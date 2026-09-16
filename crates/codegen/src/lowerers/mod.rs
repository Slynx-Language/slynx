mod components;
mod expressions;
mod functions;
mod instructions;
mod types;

use std::collections::{HashMap, HashSet};

use common::{FrontendSymbol, SymbolPointer};
use slynx_hir::{
    DeclarationId, HirComponentDeclaration, HirFunctionDeclaration, HirStaticDeclaration,
    HirStylesheetDeclaration, HirType, SlynxHir,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    ownership::OwnershipAnalysis,
};
use slynx_ir::{
    Component, Function, GlobalValue, IRPointer, IRStorage, IRTypeId, InitValue, SlynxIR,
};

pub use types::*;

use crate::CodegenError;

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

    pub fn generate(
        mut self,
        deadcode: HashSet<AnyDeclarationId>,
        ownership: OwnershipAnalysis,
    ) -> Result<SlynxIR, CodegenError> {
        self.ownership = ownership;
        let mut ir = SlynxIR::new();
        self.hoist_declarations(&mut ir, &deadcode)?;
        self.stylesheet_pre_pass(&mut ir, &deadcode);
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
                if let HirType::Reference { rf, .. } = &self.hir.types[declaration.ty] {
                    self.types.register_mapping(*rf, obj);
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

    /// Pre-pass: compute property codes for all stylesheets.
    fn stylesheet_pre_pass(&mut self, _ir: &mut SlynxIR, deadcode: &HashSet<AnyDeclarationId>) {
        for file in self.hir.store.files.iter() {
            for (id, declaration) in file.declarations.declarations.styles.iter().with_ids() {
                if deadcode.contains(&AnyDeclarationId::new(
                    file.file,
                    AnyLocalDeclarationId::Style(id),
                )) {
                    continue;
                }
                let HirStylesheetDeclaration {
                    usages, statements, ..
                } = declaration;

                let own_props = self.collect_style_properties(statements);
                let resolved = self.resolve_style_inheritance(usages, &own_props);
                if let Some(style_data) = self.styles.get_mut(&DeclarationId::new(file.file, id)) {
                    style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();
                }
            }
        }
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
                self.types.insert_enum_fields_for(declaration.ty, ir)?;
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
                self.types.insert_object_fields_for(obj.ty, ir)?;
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
                    ir,
                )?;
            }
            for (id, statik) in file.statik.iter().with_ids() {
                let name = statik.name;
                let ty = self.types.get_or_create_ir_type(statik.ty, ir)?;
                let id = DeclarationId::new(file.file, id);
                if statik.external {
                    self.external_statics.insert(id, ty);
                } else {
                    let global = ir.create_global(self.hir.get_name(name), InitValue::ZeroInit(ty));
                    self.globals.insert(id, global);
                }
            }
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

                    self.initialize_function(*function_ptr, declaration.ty, statements, args, ir)?;
                }
            }
        }
        Ok(())
    }
}
