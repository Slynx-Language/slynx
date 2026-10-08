use slynx_parser::{ComponentDeclaration, ComponentMemberKind, TypeContext};

use crate::{
    HIRError, HirNode, HirQueueBuilder, Result, builders::lowering::ASTLowerer, term::TermId,
};

impl<'a> ASTLowerer<'a> {
    /// Pure computation of a component's signature type (no cycle detection).
    fn compute_component_type(
        &self,
        queue: &HirQueueBuilder<'a>,
        requester: module_loader::FileId,
        component: &ComponentDeclaration,
    ) -> Result<TermId> {
        let context = TypeContext::new(&component.generics.type_params);
        let (properties, children) = {
            let mut properties = Vec::with_capacity(component.members.len());
            let mut components = Vec::with_capacity(component.members.len());
            for member in &component.members {
                match &member.kind {
                    ComponentMemberKind::Property { name, ty, .. } => {
                        if let Some(ty) = ty {
                            let field = self.lower_type(queue, requester, *ty, &context)?.term;
                            properties.push((*name, field));
                        } else {
                            return Err(HIRError::component_missing_prop_type(member.span));
                        }
                    }
                    ComponentMemberKind::Child(c) => {
                        let ty = self
                            .lower_type(queue, requester, c.data.name, &context)?
                            .term;
                        let ty_view = queue.hir.view(ty);
                        let view = ty_view.dereference();
                        if let Some(view) = view.nominal().is_component() {
                            components.push(view.data);
                        } else {
                            let name = queue.type_name(c.data.name.data);
                            return Err(HIRError::not_a_component(name, c.span));
                        };
                    }
                }
            }
            (properties, components)
        };
        Ok(queue
            .hir
            .types
            .create_component_type(component.name, properties, children))
    }

    /// Resolve a component's signature with cycle detection.
    pub(super) fn resolve_component_signature(
        &self,
        queue: &HirQueueBuilder<'a>,
        node: &HirNode,
        component: &ComponentDeclaration,
    ) -> Result<TermId> {
        let key = (node.entry, component.name);

        // Push onto cycle-detection stack
        node.pendings.signature_stack.borrow_mut().push(key);

        // Insert into in-progress set. If already present, we have a cycle.
        if !node.pendings.signatures_in_progress.insert(key) {
            let chain = node.pendings.signature_stack.borrow().clone();
            node.pendings.signature_stack.borrow_mut().pop();
            return Err(HIRError::cyclic_component_signature(
                component.name,
                chain,
                component.span,
            ));
        }

        let result = self.compute_component_type(queue, node.entry, component);

        node.pendings.signatures_in_progress.remove(&key);
        node.pendings.signature_stack.borrow_mut().pop();
        result
    }
}
