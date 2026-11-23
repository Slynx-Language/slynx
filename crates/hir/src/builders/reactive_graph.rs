use common::pool::PoolId;
use dashmap::DashMap;
use petgraph::{Graph, acyclic::Acyclic, data::Build, graph::NodeIndex};

use crate::{
    ComponentMemberDeclaration, DeclarationId, HIRError, HirComponentDeclaration, HirExpression,
    HirExpressionKind, HirQueueBuilder, Result, SlynxHir,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DagNode {
    ///A property from a component. The provided `index` is the index of it on the component
    Property { index: usize },
    ///An input from a component. The provided `index` is the index of it on the component
    Input { index: usize },
    ///A view from a component. The provided `index` is the index of it on the component
    View { index: usize },
    ///A child field from a component. The provided `child` is the index of the child of the component and the field is the index of its field(ideally an input, but may be a property)
    ChildField { child: usize, field: usize },
}

#[derive(Debug, Clone)]
pub struct ReactiveGraph {
    pub graph: Acyclic<Graph<DagNode, ()>>,
    pub node_indices: DashMap<DagNode, NodeIndex>,
}

impl ReactiveGraph {
    pub fn new() -> Self {
        Self {
            graph: Acyclic::new(),
            node_indices: DashMap::new(),
        }
    }

    fn node_for(&mut self, node: DagNode) -> NodeIndex {
        if let Some(existing) = self.node_indices.get(&node) {
            return *existing;
        }
        let idx = self.graph.add_node(node);
        self.node_indices.insert(node, idx);
        idx
    }
}

impl HirQueueBuilder<'_> {
    pub(crate) fn enqueue_dag(
        &self,
        decl: DeclarationId<HirComponentDeclaration>,
    ) -> Result<ReactiveGraph> {
        let mut graph = ReactiveGraph::new();

        let Some(members) = self.resolved_components.get(&decl) else {
            self.hir.component_graphs.insert(decl, graph.clone());
            return Ok(graph);
        };

        // First, create a node for every component member that can act as a
        // reactive source or computed value.
        for member in members.iter() {
            match member {
                ComponentMemberDeclaration::Property { index, .. } => {
                    graph.node_for(DagNode::Property { index: *index });
                }
                ComponentMemberDeclaration::Input { index, .. } => {
                    graph.node_for(DagNode::Input { index: *index });
                }
                ComponentMemberDeclaration::View { index, .. } => {
                    graph.node_for(DagNode::View { index: *index });
                }
                _ => {}
            }
        }

        // Views depend on the members they read. Connect those dependencies.
        for member in members.iter() {
            if let ComponentMemberDeclaration::View { index, rhs, .. } = member {
                let mut refs = Vec::new();
                collect_member_refs(self.hir, rhs.data, &mut refs);
                refs.sort();
                refs.dedup();
                let view_node = graph.node_for(DagNode::View { index: *index });
                for source in refs {
                    if let Some(source_node) = member_node_for(source, &*members, &mut graph) {
                        if graph
                            .graph
                            .try_add_edge(source_node, view_node, ())
                            .is_err()
                        {
                            let component =
                                &self.hir.get_file(decl.owner).declarations.components[decl.term];
                            return Err(HIRError::cyclic_reactive_dependency(
                                decl.owner,
                                component.name,
                                component.span,
                            ));
                        }
                    }
                }
            }
        }

        // Child property assignments may depend on component members. Each such
        // dependency becomes an edge from the source member to the child field.
        let mut child_index = 0usize;
        for member in members.iter() {
            if let ComponentMemberDeclaration::Child(component_expr) = member {
                let expr = &self.hir[component_expr.data];
                for property in &expr.properties {
                    let field = property.index();
                    let target = graph.node_for(DagNode::ChildField {
                        child: child_index,
                        field,
                    });

                    let mut refs = Vec::new();
                    collect_member_refs(self.hir, property.expr().data, &mut refs);
                    refs.sort();
                    refs.dedup();
                    for source in refs {
                        if let Some(source_node) = member_node_for(source, &*members, &mut graph) {
                            if graph.graph.try_add_edge(source_node, target, ()).is_err() {
                                let component =
                                    &self.hir.get_file(decl.owner).declarations.components
                                        [decl.term];
                                return Err(HIRError::cyclic_reactive_dependency(
                                    decl.owner,
                                    component.name,
                                    component.span,
                                ));
                            }
                        }
                    }
                }
                child_index += 1;
            }
        }

        self.hir.component_graphs.insert(decl, graph.clone());
        Ok(graph)
    }
}

fn member_node_for(
    source_index: usize,
    members: &[ComponentMemberDeclaration],
    graph: &mut ReactiveGraph,
) -> Option<NodeIndex> {
    // The member index alone identifies a component member because member
    // indices are assigned in declaration order.
    for member in members.iter() {
        match member {
            ComponentMemberDeclaration::Property { index, .. }
            | ComponentMemberDeclaration::Input { index, .. }
            | ComponentMemberDeclaration::View { index, .. }
                if *index == source_index =>
            {
                let node = match member {
                    ComponentMemberDeclaration::Property { index, .. } => {
                        DagNode::Property { index: *index }
                    }
                    ComponentMemberDeclaration::Input { index, .. } => {
                        DagNode::Input { index: *index }
                    }
                    ComponentMemberDeclaration::View { index, .. } => {
                        DagNode::View { index: *index }
                    }
                    _ => return None,
                };
                return Some(graph.node_for(node));
            }
            _ => {}
        }
    }
    None
}

fn collect_member_refs(hir: &SlynxHir, expr_id: PoolId<HirExpression>, refs: &mut Vec<usize>) {
    let expr = &hir[expr_id];
    match &expr.kind {
        HirExpressionKind::Identifier(var) if var.is_component() => {
            refs.push(var.index as usize);
        }
        HirExpressionKind::FieldAccess { expr: parent, .. } => {
            collect_member_refs(hir, parent.data, refs);
        }
        HirExpressionKind::Binary { lhs, rhs, .. } => {
            collect_member_refs(hir, lhs.data, refs);
            collect_member_refs(hir, rhs.data, refs);
        }
        HirExpressionKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_member_refs(hir, arg.data, refs);
            }
        }
        HirExpressionKind::Component(component) => {
            let component_expr = &hir[component.data];
            for property in &component_expr.properties {
                collect_member_refs(hir, property.expr().data, refs);
            }
        }
        HirExpressionKind::Object { fields, .. } => {
            for field in fields {
                collect_member_refs(hir, field.data, refs);
            }
        }
        HirExpressionKind::Array(items) | HirExpressionKind::Vector(items) => {
            for item in items {
                collect_member_refs(hir, item.data, refs);
            }
        }
        HirExpressionKind::Tuple(items) => {
            for item in items {
                collect_member_refs(hir, item.data, refs);
            }
        }
        HirExpressionKind::ArrayIndex(target, _) => {
            collect_member_refs(hir, target.data, refs);
        }
        HirExpressionKind::Deref(inner) | HirExpressionKind::Reference(inner) => {
            collect_member_refs(hir, inner.data, refs);
        }
        HirExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_member_refs(hir, condition.data, refs);
            for stmt in then_branch {
                collect_stmt_member_refs(hir, stmt.data, refs);
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    collect_stmt_member_refs(hir, stmt.data, refs);
                }
            }
        }
        HirExpressionKind::Matches { value, args, .. } => {
            collect_member_refs(hir, value.data, refs);
            for arg in args {
                collect_member_refs(hir, arg.data, refs);
            }
        }
        HirExpressionKind::Enum { args, .. } => {
            for arg in args {
                collect_member_refs(hir, arg.data, refs);
            }
        }
        _ => {}
    }
}

fn collect_stmt_member_refs(
    hir: &SlynxHir,
    stmt_id: PoolId<crate::HirStatement>,
    refs: &mut Vec<usize>,
) {
    let stmt = &hir[stmt_id];
    match stmt {
        crate::HirStatement::Variable { value, .. } => {
            collect_member_refs(hir, value.data, refs);
        }
        crate::HirStatement::Assign { lhs, value } => {
            collect_member_refs(hir, lhs.data, refs);
            collect_member_refs(hir, value.data, refs);
        }
        crate::HirStatement::Expression { expr } => {
            collect_member_refs(hir, expr.data, refs);
        }
        crate::HirStatement::Return { expr } => {
            if let Some(expr) = expr {
                collect_member_refs(hir, expr.data, refs);
            }
        }
        crate::HirStatement::While { condition, body } => {
            collect_member_refs(hir, condition.data, refs);
            for stmt in body {
                collect_stmt_member_refs(hir, stmt.data, refs);
            }
        }
    }
}
