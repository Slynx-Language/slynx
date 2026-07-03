mod components;
mod error;
mod expressions;
mod functions;
mod helper;
mod instructions;
mod queries;
use std::{collections::HashMap, ops::Deref};

use common::{FrontendSymbol, SymbolPointer, pool::DedupPoolId};
pub use error::*;
use helper::styles::StyleData;
use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use slynx_hir::{
    DeclarationId, HirComponentDeclaration, HirFunctionDeclaration, HirStaticDeclaration,
    HirStylesheetDeclaration, HirType, SlynxHir,
};
use slynx_ir::{
    Component, Function, GlobalValue, IRPointer, IRStorage, IRTypeId, InitValue, SlynxIR,
};

/// Per-component data for emitting child initcalls at instantiation time.
pub(crate) struct ChildInitWork {
    pub children_type: Vec<IRTypeId>,
    ///Index of the child inside the component
    pub children_index: Vec<usize>,
    pub init_func: IRPointer<Function, 1>,
    /// Indices into the parent component's property list.
    pub parent_prop_indices: Vec<usize>,
}
pub type TypeId = DedupPoolId<HirType>;

pub struct Codegen {
    external_statics: HashMap<DeclarationId<HirStaticDeclaration>, IRTypeId>,
    globals: HashMap<DeclarationId<HirStaticDeclaration>, IRPointer<GlobalValue, 1>>,
    names: HashMap<SymbolPointer<FrontendSymbol>, SymbolPointer<SlynxIR>>,
    types: HashMap<TypeId, IRTypeId>,
    functions: HashMap<DeclarationId<HirFunctionDeclaration>, IRPointer<Function, 1>>,
    components: HashMap<DeclarationId<HirComponentDeclaration>, IRPointer<Component, 1>>,
    styles: HashMap<DeclarationId<HirStylesheetDeclaration>, StyleData>,
    /// Child-init work items queued by `initialize_component` and
    /// executed by `get_component_expression`.
    pub(crate) component_child_inits: HashMap<TypeId, Vec<ChildInitWork>>,
}

impl Default for Codegen {
    fn default() -> Self {
        Self::new()
    }
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            external_statics: HashMap::new(),
            globals: HashMap::new(),
            names: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
            components: HashMap::new(),
            styles: HashMap::new(),
            component_child_inits: HashMap::new(),
        }
    }

    /// Interns a HIR symbol into the IR and caches the mapping.
    pub(crate) fn intern_to_ir(
        &mut self,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
        symbol: SymbolPointer<FrontendSymbol>,
    ) -> SymbolPointer<SlynxIR> {
        let s = hir.get_name(symbol);
        let ptr = ir.strings.intern(s);
        self.names.insert(symbol, ptr);
        ptr
    }

    pub fn generate(&mut self, hir: &SlynxHir) -> Result<SlynxIR, CodegenError> {
        let mut ir = SlynxIR::new();
        self.hoist_declarations(hir, &mut ir);
        self.stylesheet_pre_pass(hir, &mut ir);
        self.lower_non_stylesheets(hir, &mut ir)?;
        self.lower_stylesheets(hir, &mut ir)?;
        Ok(ir)
    }

    /// Phase 0: Hoist declarations.
    fn hoist_declarations(&mut self, hir: &SlynxHir, ir: &mut SlynxIR) {
        for file in &hir.files {
            for (id, declaration) in file.declarations.objects.iter().with_ids() {
                let obj = ir.create_struct(hir.get_name(declaration.name));
                self.types.insert(declaration.ty, obj);
                // declaration.ty is a Reference; also register the concrete
                // Struct TypeId so tuple fields (which resolve through the
                // Reference) can be found in get_or_create_ir_type.
                if let HirType::Reference { rf, .. } = &hir.deref()[declaration.ty] {
                    self.types.insert(*rf, obj);
                }
            }
            for (id, declaration) in file.declarations.functions.iter().with_ids() {
                let name = hir.get_name(declaration.name);
                let ptr = ir.create_function(name);
                let ty = ir.get(ptr).ty();
                self.types.insert(declaration.ty, ty);
                self.functions
                    .insert(DeclarationId::new(file.file, id), ptr);
            }
            for (id, declaration) in file.declarations.components.iter().with_ids() {
                let comp_name = hir.get_name(declaration.name);
                let component = ir.create_component(comp_name);
                let component_ty = ir.get(component).ir_type();
                self.types.insert(declaration.ty, component_ty);
                self.components
                    .insert(DeclarationId::new(file.file, id), component);
            }
            for (id, declaration) in file.declarations.styles.iter().with_ids() {
                let name = hir.get_name(declaration.name);
                let init_func = ir.create_function(&format!("__init_{name}"));
                let apply_func = ir.create_function(&format!("__apply_{name}"));
                let struct_ty = ir.create_struct(&format!("__{name}_struct"));
                self.types.insert(declaration.ty, struct_ty);
                self.styles.insert(
                    DeclarationId::new(file.file, id),
                    StyleData {
                        init_func,
                        apply_func,
                        struct_ty,
                        property_codes: Vec::new(),
                    },
                );
            }
        }
    }

    /// Pre-pass: compute property codes for all stylesheets.
    fn stylesheet_pre_pass(&mut self, hir: &SlynxHir, _ir: &mut SlynxIR) {
        for file in hir.files.iter() {
            for (id, declaration) in file.declarations.declarations.styles.iter().with_ids() {
                let HirStylesheetDeclaration {
                    usages, statements, ..
                } = declaration;

                let own_props = self.collect_style_properties(statements);
                let resolved = self.resolve_style_inheritance(usages, &own_props, hir);
                if let Some(style_data) = self.styles.get_mut(&DeclarationId::new(file.file, id)) {
                    style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();
                }
            }
        }
    }

    /// Phase 1: Lower all non-stylesheet declarations.
    fn lower_non_stylesheets(
        &mut self,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        for file in &hir.files {
            for obj in file.declarations.objects.iter() {
                self.insert_object_fields_for(obj.ty, hir, ir)?;
            }
            for (id, component) in file.declarations.components.iter().with_ids() {
                self.initialize_component(
                    DeclarationId::new(file.file, id),
                    component,
                    &component.props,
                    hir,
                    ir,
                )?;
            }
            for (id, statik) in file.statik.iter().with_ids() {
                let name = statik.name;
                let ty = self.get_or_create_ir_type(&statik.ty, hir, ir)?;
                let id = DeclarationId::new(file.file, id);
                if statik.external {
                    self.external_statics.insert(id, ty);
                } else {
                    let global = ir.create_global(hir.get_name(name), InitValue::ZeroInit(ty));
                    self.globals.insert(id, global);
                }
            }
            for (id, declaration) in file.declarations.functions.iter().with_ids() {
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
                        hir,
                        ir,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Phase 2: Lower stylesheets in dependency order.
    fn lower_stylesheets(&mut self, hir: &SlynxHir, ir: &mut SlynxIR) -> Result<(), CodegenError> {
        let (flat_decls, decl_to_idx): (Vec<_>, _) = {
            let mut decls = Vec::new();
            let mut idx = HashMap::new();
            for file in &hir.files {
                for (id, decl) in file.value().declarations.styles.iter().with_ids() {
                    let id = DeclarationId::new(file.file, id);
                    idx.insert(id, decls.len());
                    decls.push(id);
                }
            }
            (decls, idx)
        };
        if flat_decls.is_empty() {
            return Ok(());
        }

        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        let mut node_indices: HashMap<usize, NodeIndex<u32>> = HashMap::new();

        for (idx, _) in flat_decls.iter().enumerate() {
            node_indices.insert(idx, graph.add_node(idx));
        }

        for (idx, id) in flat_decls.iter().enumerate() {
            let reader = hir.get_file(id.file_id);
            let HirStylesheetDeclaration { usages, .. } = &reader[id.local_id];

            for usage in usages {
                let parent_idx = decl_to_idx[&usage.style];
                if let Some(&parent_node) = node_indices.get(&parent_idx) {
                    graph.add_edge(parent_node, node_indices[&idx], ());
                }
            }
        }

        let order = match toposort(&graph, None) {
            Ok(order) => order.into_iter().map(|n| graph[n]).collect::<Vec<_>>(),
            Err(_) => (0..flat_decls.len()).collect(),
        };

        for &idx in &order {
            let id = flat_decls[idx];
            let reader = hir.get_file(id.file_id);
            self.lower_stylesheet(id, &reader[id.local_id], hir, ir)?;
        }
        Ok(())
    }
}
