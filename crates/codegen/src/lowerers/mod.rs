mod components;
mod expressions;
mod functions;
mod instructions;
mod types;

use std::collections::{HashMap, HashSet};

use common::{FrontendSymbol, SymbolPointer};
use slynx_hir::{
    DeclarationId, FileId, HirComponentDeclaration, HirFunctionDeclaration, HirStaticDeclaration,
    SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    ownership::OwnershipAnalysis,
    term::TermNode,
};
use slynx_ir::{
    Component, Function, GlobalValue, IRPointer, IRStorage, IRTypeId, InitValue, SlynxIR,
};

pub use types::*;

use crate::CodegenError;

///The IR storage backing a static declaration.
///
///A static is materialized the first time an expression reads it, so this is
///also what distinguishes "not referenced yet" from "extern".
#[derive(Debug, Clone, Copy)]
pub enum StaticStorage {
    ///An `extern` static has no IR definition at all — the backend resolves
    ///the name at link time. Only its type and its external name are needed.
    Extern(IRTypeId),
    ///A plain static becomes a zero-initialized global.
    Global(IRPointer<GlobalValue, 1>),
}

///The shared state of the HIR→IR lowering pipeline.
///
///All lowerers operate through this struct instead of threading `&SlynxHir`
///around: it owns the type lowering (`types`), the name interning cache, and the
///hoisted IR handles (functions, components, globals, styles) plus the ownership
///analysis results. Lowering methods read the HIR through
///[`LoweringState::hir`] and write IR through the `&mut SlynxIR` handed to each
///phase, so no lowerer ever reaches into a raw `SlynxHir`.
pub struct LoweringState<'a> {
    ///The HIR being lowered.
    pub hir: &'a SlynxHir<'a>,
    ///HIR→IR type layout lowering (types, enum layouts, mapped structs).
    pub types: TypeLowerer<'a>,
    ///HIR symbol → IR symbol interning cache.
    names: HashMap<SymbolPointer<FrontendSymbol>, SymbolPointer<SlynxIR>>,
    ///IR types assigned to hoisted external statics.
    external_statics: HashMap<DeclarationId<HirStaticDeclaration>, IRTypeId>,
    ///IR globals for each non-external static.
    globals: HashMap<DeclarationId<HirStaticDeclaration>, IRPointer<GlobalValue, 1>>,
    ///Hoisted IR functions keyed by HIR declaration.
    functions: HashMap<DeclarationId<HirFunctionDeclaration>, IRPointer<Function, 1>>,
    ///Hoisted IR components keyed by HIR declaration.
    components: HashMap<DeclarationId<HirComponentDeclaration>, IRPointer<Component, 1>>,
    ///Ownership analysis results for move/copy/borrow tracking.
    ownership: OwnershipAnalysis,
}

impl<'a> LoweringState<'a> {
    pub fn new(hir: &'a SlynxHir<'a>) -> Self {
        Self {
            hir,
            types: TypeLowerer::new(hir),
            names: HashMap::new(),
            external_statics: HashMap::new(),
            globals: HashMap::new(),
            functions: HashMap::new(),
            components: HashMap::new(),
            ownership: OwnershipAnalysis::new(),
        }
    }

    /// Interns a HIR symbol into the IR and caches the mapping.
    pub(crate) fn intern_to_ir(
        &mut self,
        ir: &mut SlynxIR,
        symbol: SymbolPointer<FrontendSymbol>,
    ) -> SymbolPointer<SlynxIR> {
        let s = self.hir.get_name(symbol);
        let ptr = ir.strings.intern(s);
        self.names.insert(symbol, ptr);
        ptr
    }

    ///Resolves the IR storage for the static `id`, materializing it on first
    ///reference.
    ///
    ///Statics are resolved lazily rather than in a hoisting pass because a
    ///static may be declared in a file that has not been walked yet:
    ///declarations are visited through `HirStore::files`, which is a `DashMap`
    ///and so has no stable order, and registering per-file while lowering bodies
    ///per-file made a cross-file reference fail or succeed depending on
    ///iteration order. Resolving at the single point where a static is read
    ///removes the ordering constraint instead of papering over it.
    ///
    ///A static that is never referenced is never created, which is why this needs
    ///no `deadcode` check: unreferenced statics never reach here.
    fn ensure_static(
        &mut self,
        id: DeclarationId<HirStaticDeclaration>,
        ir: &mut SlynxIR,
        file: FileId,
    ) -> Result<StaticStorage, CodegenError> {
        if let Some(ty) = self.external_statics.get(&id) {
            return Ok(StaticStorage::Extern(*ty));
        }
        if let Some(global) = self.globals.get(&id) {
            return Ok(StaticStorage::Global(*global));
        }

        // `DeclarationId` is `(FileId, PoolId<…>)`, so this is a direct index
        // into the owning file's pool — no search involved.
        let statik = &self.hir.get_file(id.owner)[id.term];
        let ty = self
            .types
            .get_or_create_ir_type(statik.ty.term, statik.span, ir, file)?;

        let storage = if statik.external {
            self.external_statics.insert(id, ty);
            StaticStorage::Extern(ty)
        } else {
            let global = ir.create_global(self.hir.get_name(statik.name), InitValue::ZeroInit(ty));
            self.globals.insert(id, global);
            StaticStorage::Global(global)
        };
        Ok(storage)
    }

    pub fn generate(
        mut self,
        deadcode: HashSet<AnyDeclarationId>,
        ownership: OwnershipAnalysis,
    ) -> Result<SlynxIR, CodegenError> {
        self.ownership = ownership;
        let mut ir = SlynxIR::new();
        self.hoist_declarations(&mut ir, &deadcode)?;
        self.lower_non_stylesheets(&mut ir, &deadcode)?;
        Ok(ir)
    }

    /// Phase 0: Hoist declarations.
    fn hoist_declarations(
        &mut self,
        ir: &mut SlynxIR,
        deadcode: &HashSet<AnyDeclarationId>,
    ) -> Result<(), CodegenError> {
        for file in &self.hir.store.files {
            for (id, declaration) in file.declarations.objects.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Object(id),
                )) {
                    continue;
                }
                let obj = ir.create_struct(self.hir.get_name(declaration.name));
                self.types.register_mapping(declaration.ty, obj);
                // declaration.ty is a Reference; also register the concrete
                // Struct TypeId so tuple fields (which resolve through the
                // Reference) can be found in get_or_create_ir_type.
                if let TermNode::Apply { target, .. } = self.hir.view(declaration.ty).raw().node()
                    && self.hir.view(*target).is_extension().is_none()
                {
                    self.types.register_mapping(*target, obj);
                }
            }
            for (id, declaration) in file.declarations.functions.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Function(id),
                )) {
                    continue;
                }
                let name = self.hir.get_name(declaration.name);
                let ptr = ir.create_function(name, declaration.external);
                let ty = ir.get(ptr).ty();
                self.types.register_mapping(declaration.ty, ty);
                self.functions
                    .insert(DeclarationId::new(file.file, id), ptr);
            }
            for (id, declaration) in file.declarations.enums.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Enum(id),
                )) {
                    continue;
                }
                let name = self.hir.get_name(declaration.name);
                let enum_struct = ir.create_struct(name);
                self.types.register_mapping(declaration.ty, enum_struct);
            }
            for (id, declaration) in file.declarations.components.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Component(id),
                )) {
                    continue;
                }
                let comp_name = self.hir.get_name(declaration.name);
                let component = ir.create_component(comp_name);
                let component_ty = ir.get(component).ir_type();
                self.types.register_mapping(declaration.ty, component_ty);
                self.components
                    .insert(DeclarationId::new(file.file, id), component);
            }
        }
        Ok(())
    }

    /// Phase 1: Lower all non-stylesheet declarations.
    fn lower_non_stylesheets(
        &mut self,
        ir: &mut SlynxIR,
        deadcode: &HashSet<AnyDeclarationId>,
    ) -> Result<(), CodegenError> {
        // Fill enum struct fields across all files before any function body is
        // lowered, so payload references to enums in other files resolve.
        for file in &self.hir.store.files {
            for (id, declaration) in file.declarations.enums.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Enum(id),
                )) {
                    continue;
                }
                self.types.insert_enum_fields_for(
                    declaration.ty,
                    declaration.span,
                    ir,
                    file.file,
                )?;
            }
        }
        for file in &self.hir.store.files {
            for (id, obj) in file.declarations.objects.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Object(id),
                )) {
                    continue;
                }
                self.types
                    .insert_object_fields_for(obj.ty, obj.span, ir, file.file)?;
            }
            for (id, component) in file.declarations.components.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Component(id),
                )) {
                    continue;
                }
                self.initialize_component(
                    DeclarationId::new(file.file, id),
                    component,
                    &component.props,
                    component.span,
                    ir,
                    file.file,
                )?;
            }
            // Statics are deliberately absent here. They are materialized by
            // `ensure_static` at the point an expression reads one, so that a
            // static declared in a not-yet-visited file still resolves.
            for (id, declaration) in file.declarations.functions.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Function(id),
                )) {
                    continue;
                }
                let HirFunctionDeclaration {
                    statements, args, ..
                } = declaration;
                {
                    let function_ptr = self
                        .functions
                        .get(&DeclarationId::new(file.file, id))
                        .expect("Function should have been hoisted");

                    self.initialize_function(
                        *function_ptr,
                        declaration.ty,
                        statements,
                        args,
                        declaration.span,
                        ir,
                        file.file,
                    )?;
                }
            }
        }
        Ok(())
    }
}
