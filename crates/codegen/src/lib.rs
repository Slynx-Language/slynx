mod components;
mod contexts;
mod error;
mod helper;
mod instructions;
mod temporary_data;
use std::collections::HashMap;

pub use error::*;
use petgraph::{
    algo::toposort,
    graph::{DiGraph, NodeIndex},
};
use slynx_hir::{HirDeclaration, HirDeclarationKind, HirType, SlynxHir};
use slynx_ir::SlynxIR;

use crate::temporary_data::TempIRData;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct Codegen {
    hir: SlynxHir,
    ir: SlynxIR,
    temp: TempIRData,
}

impl Codegen {
    pub fn new(hir: SlynxHir) -> Self {
        Self {
            ir: SlynxIR::new(),
            temp: TempIRData::new(),
            hir,
        }
    }

    ///Generates all the code on the IR, with types, functions, lowerings, etc, based on the provided `hir`. The `tys` is expected to be the types module used by the `hir` during all frontend process, as well as
    ///the `symbols`, to be the symbols module used by the same `hir` during all the frontend process
    pub fn generate(&mut self, hir: Vec<HirDeclaration>) -> Result<(), CodegenError> {
        self.hoist_declarations(&hir);
        self.stylesheet_pre_pass(&hir);
        self.lower_non_stylesheets(&hir)?;
        self.lower_stylesheets(&hir)?;

        Ok(())
    }

    /// Phase 0: Hoist all declarations into the IR.
    /// Creates placeholder IR types, functions, components, and style scaffolding
    /// so later phases can reference them by ID.
    fn hoist_declarations(&mut self, hir: &[HirDeclaration]) {
        for declaration in hir {
            match &declaration.kind {
                HirDeclarationKind::Object => {}
                HirDeclarationKind::Function { name, .. } => {}
                HirDeclarationKind::ComponentDeclaration { name, .. } => {}
                HirDeclarationKind::StyleSheet { .. } => {}
                HirDeclarationKind::Alias => {}
            }
        }
    }

    /// Pre-pass: compute property codes for all stylesheets before any lowering.
    /// This ensures parent property_codes are available regardless of declaration order.
    fn stylesheet_pre_pass(&mut self, hir: &[HirDeclaration]) {
        for declaration in hir {
            if let HirDeclarationKind::StyleSheet {
                ref usages,
                ref statements,
                ..
            } = declaration.kind
            {
                let own_props = self.collect_style_properties(statements);
                let resolved = self.resolve_style_inheritance(usages, &own_props, hir);
                if let Some(style_data) = self.temp.get_style_mut(declaration.id) {
                    style_data.property_codes = resolved.iter().map(|rp| rp.property).collect();
                }
            }
        }
    }

    /// Phase 1: Lower all non-stylesheet declarations (Objects, Functions, Components, Aliases).
    fn lower_non_stylesheets(&mut self, hir: &[HirDeclaration]) -> Result<(), CodegenError> {
        for declaration in hir {
            match &declaration.kind {
                HirDeclarationKind::Object => {
                    self.insert_object_fields_for(declaration.ty)?;
                }
                HirDeclarationKind::Function {
                    args, statements, ..
                } => {}
                HirDeclarationKind::ComponentDeclaration { props, .. } => {
                    self.initialize_component(declaration, props)?;
                }
                HirDeclarationKind::Alias => {}
                HirDeclarationKind::StyleSheet { .. } => {}
            }
        }
        Ok(())
    }

    /// Phase 2: Lower stylesheets in dependency order (parents before children).
    ///
    /// Uses `petgraph::algo::toposort` to compute a valid lowering order from the
    /// `uses` dependency graph.
    fn lower_stylesheets(&mut self, hir: &[HirDeclaration]) -> Result<(), CodegenError> {
        let all_stylesheets: Vec<usize> = (0..hir.len())
            .filter(|i| matches!(hir[*i].kind, HirDeclarationKind::StyleSheet { .. }))
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
            if let HirDeclarationKind::StyleSheet { ref usages, .. } = hir[idx].kind {
                for usage in usages {
                    let parent_idx = usage.style.as_raw() as usize;
                    if let Some(&parent_node) = node_indices.get(&parent_idx) {
                        graph.add_edge(parent_node, node_indices[&idx], ());
                    }
                }
            }
        }

        let order = match toposort(&graph, None) {
            Ok(order) => order.into_iter().map(|n| graph[n]).collect::<Vec<_>>(),
            Err(_) => {
                // Cycle detected: fall back to lowering all remaining stylesheets
                all_stylesheets
            }
        };

        for &idx in &order {
            self.lower_stylesheet(&hir[idx])?;
        }

        Ok(())
    }
}
