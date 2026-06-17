mod components;
mod error;
mod expressions;
mod functions;
mod helper;
mod instructions;
mod queries;
use std::collections::HashMap;

use common::SymbolPointer;
pub use error::*;
use helper::styles::StyleData;
use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use slynx_hir::{DeclarationId, HirDeclarationKind, HirType, SlynxHir, TypeId};
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

pub struct Codegen {
    external_statics: HashMap<DeclarationId, IRTypeId>,
    globals: HashMap<DeclarationId, IRPointer<GlobalValue, 1>>,
    names: HashMap<SymbolPointer<SlynxHir>, SymbolPointer<SlynxIR>>,
    types: HashMap<TypeId, IRTypeId>,
    functions: HashMap<DeclarationId, IRPointer<Function, 1>>,
    components: HashMap<DeclarationId, IRPointer<Component, 1>>,
    styles: HashMap<DeclarationId, StyleData>,
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
        symbol: SymbolPointer<SlynxHir>,
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
            let file = file.read();
            for declaration in file.declarations() {
                let declaration = declaration.1;

                match &declaration.kind {
                    HirDeclarationKind::Static => {}
                    HirDeclarationKind::Object => {
                        let name = hir.get_declaration_name(declaration.id);
                        let obj = ir.create_struct(name);
                        self.types.insert(declaration.ty, obj);
                        // declaration.ty is a Reference; also register the concrete
                        // Struct TypeId so tuple fields (which resolve through the
                        // Reference) can be found in get_or_create_ir_type.
                        if let HirType::Reference { rf, .. } = &*hir.get_type(&declaration.ty) {
                            self.types.insert(*rf, obj);
                        }
                    }
                    HirDeclarationKind::Function { name, .. } => {
                        let name = hir.get_name(*name);
                        let ptr = ir.create_function(name);
                        let ty = ir.get(ptr).ty();
                        self.types.insert(declaration.ty, ty);
                        self.functions.insert(declaration.id, ptr);
                    }
                    HirDeclarationKind::ComponentDeclaration { name, .. } => {
                        let comp_name = hir.get_name(*name);
                        let component = ir.create_component(comp_name);
                        let component_ty = ir.get(component).ir_type();
                        self.types.insert(declaration.ty, component_ty);
                        self.components.insert(declaration.id, component);
                    }
                    HirDeclarationKind::StyleSheet { .. } => {
                        let name = hir.get_declaration_name(declaration.id);
                        let init_func = ir.create_function(&format!("__init_{name}"));
                        let apply_func = ir.create_function(&format!("__apply_{name}"));
                        let struct_ty = ir.create_struct(&format!("__{name}_struct"));
                        self.types.insert(declaration.ty, struct_ty);
                        self.styles.insert(
                            declaration.id,
                            StyleData {
                                init_func,
                                apply_func,
                                struct_ty,
                                property_codes: Vec::new(),
                            },
                        );
                    }
                    HirDeclarationKind::Alias => {}
                }
            }
        }
    }

    /// Pre-pass: compute property codes for all stylesheets.
    fn stylesheet_pre_pass(&mut self, hir: &SlynxHir, _ir: &mut SlynxIR) {
        for file in &hir.files {
            let file = file.read();
            for declaration in file.declarations() {
                let declaration = declaration.1;
                if let HirDeclarationKind::StyleSheet {
                    ref usages,
                    ref statements,
                    ..
                } = declaration.kind
                {
                    let own_props = self.collect_style_properties(statements);
                    let resolved = self.resolve_style_inheritance(usages, &own_props, hir);
                    if let Some(style_data) = self.styles.get_mut(&declaration.id) {
                        style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();
                    }
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
            let file = file.read();
            for declaration in file.declarations() {
                let declaration = declaration.1;
                match &declaration.kind {
                    HirDeclarationKind::Static => {
                        let name = hir.get_declaration_name(declaration.id);
                        let ty = hir.get_declaration_type(declaration.id);
                        let ty = self.get_or_create_ir_type(&ty, hir, ir)?;
                        if declaration.external {
                            self.external_statics.insert(declaration.id, ty);
                        } else {
                            let global = ir.create_global(name, InitValue::ZeroInit(ty));
                            self.globals.insert(declaration.id, global);
                        }
                    }
                    HirDeclarationKind::Object => {
                        self.insert_object_fields_for(declaration.ty, hir, ir)?;
                    }
                    HirDeclarationKind::Function {
                        statements, args, ..
                    } => {
                        let function_ptr = self
                            .functions
                            .get(&declaration.id)
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
                    HirDeclarationKind::ComponentDeclaration { props, .. } => {
                        self.initialize_component(declaration, props, hir, ir)?;
                    }
                    HirDeclarationKind::Alias => {}
                    HirDeclarationKind::StyleSheet { .. } => {}
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
                let file = file.read();
                for decl in file.declarations() {
                    let declaration = decl.1;
                    idx.insert(declaration.id, decls.len());
                    decls.push(declaration.id);
                }
            }
            (decls, idx)
        };
        let all_stylesheets: Vec<usize> = (0..flat_decls.len())
            .filter(|i| {
                let id = flat_decls[*i];
                let reader = hir.get_file(id.file_id);
                let decl = reader.declarations.get_declaration(id.local_id);
                matches!(decl.kind, HirDeclarationKind::StyleSheet { .. })
            })
            .collect();

        if all_stylesheets.is_empty() {
            return Ok(());
        }

        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        let mut node_indices: HashMap<usize, NodeIndex<u32>> = HashMap::new();

        for &idx in &all_stylesheets {
            node_indices.insert(idx, graph.add_node(idx));
        }

        for &idx in &all_stylesheets {
            let id = flat_decls[idx];
            let reader = hir.get_file(id.file_id);
            let decl = reader.declarations.get_declaration(id.local_id);
            if let HirDeclarationKind::StyleSheet { ref usages, .. } = decl.kind {
                for usage in usages {
                    let parent_idx = decl_to_idx[&usage.style];
                    if let Some(&parent_node) = node_indices.get(&parent_idx) {
                        graph.add_edge(parent_node, node_indices[&idx], ());
                    }
                }
            }
        }

        let order = match toposort(&graph, None) {
            Ok(order) => order.into_iter().map(|n| graph[n]).collect::<Vec<_>>(),
            Err(_) => all_stylesheets,
        };

        for &idx in &order {
            let id = flat_decls[idx];
            let reader = hir.get_file(id.file_id);
            let decl = reader.declarations.get_declaration(id.local_id);
            self.lower_stylesheet(decl, hir, ir)?;
        }
        Ok(())
    }
}
