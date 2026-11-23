use slynx_hir::{DeclarationId, HirComponentDeclaration, SlynxHir};

use crate::Monomorphizer;

impl Monomorphizer {
    /// Copies the reactive dependency graph of a generic component template to
    /// one of its monomorphized specializations.
    ///
    /// For now the graph topology is shared unchanged: component member indices
    /// are stable across specialization, so a clone is sufficient. Later passes
    /// may replace type-dependent parts of the graph here.
    pub(crate) fn monomorphize_component_graph(
        &self,
        hir: &SlynxHir,
        template: DeclarationId<HirComponentDeclaration>,
        specialized: DeclarationId<HirComponentDeclaration>,
    ) {
        if let Some(graph) = hir.component_graphs.get(&template) {
            hir.component_graphs.insert(specialized, graph.clone());
        }
    }
}
