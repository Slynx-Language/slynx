use slynx_parser::{ComponentDeclaration, ComponentMemberKind, FuncDeclaration, TypeContext};

use crate::{
    ComponentChild, HIRError, HirNode, HirQueueBuilder, PropertyCallback, Result, SymbolPointer,
    builders::lowering::ASTLowerer, term::TermId,
};

impl<'a> ASTLowerer<'a> {
    /// Pure computation of a component's signature type (no cycle detection).
    fn compute_component_type(
        &self,
        queue: &HirQueueBuilder<'a>,
        requester: module_loader::FileId,
        component: &'a ComponentDeclaration,
    ) -> Result<TermId> {
        let context = TypeContext::new(&component.generics.type_params);
        let mut properties = Vec::with_capacity(component.members.len());

        let (callback_groups, children) = {
            let mut children = Vec::with_capacity(component.members.len());
            let mut callback_groups: Vec<(usize, SymbolPointer, TermId, Vec<&FuncDeclaration>)> =
                Vec::new();
            let mut property_index = 0;
            for member in &component.members {
                match &member.kind {
                    ComponentMemberKind::Property {
                        name,
                        ty,
                        callbacks,
                        ..
                    } => {
                        if let Some(ty) = ty {
                            let field = self.lower_type(queue, requester, *ty, &context)?.term;
                            properties.push((*name, field));
                            if !callbacks.is_empty() {
                                callback_groups.push((
                                    property_index,
                                    *name,
                                    field,
                                    callbacks.iter().collect(),
                                ));
                            }
                            property_index += 1;
                        } else {
                            return Err(HIRError::component_missing_prop_type(
                                requester,
                                member.span,
                            ));
                        }
                    }
                    ComponentMemberKind::NamedChild { name, child: c } => {
                        let ty = self
                            .lower_type(queue, requester, c.data.name, &context)?
                            .term;
                        if queue.hir.view(ty).nominal().is_component().is_some() {
                            children.push(ComponentChild {
                                name: Some(*name),
                                ty,
                            });
                        } else {
                            return Err(HIRError::not_a_component(requester, ty, c.span));
                        };
                    }
                    ComponentMemberKind::Child(c) => {
                        let ty = self
                            .lower_type(queue, requester, c.data.name, &context)?
                            .term;
                        if queue.hir.view(ty).nominal().is_component().is_some() {
                            children.push(ComponentChild { name: None, ty });
                        } else {
                            return Err(HIRError::not_a_component(requester, ty, c.span));
                        };
                    }
                }
            }
            (callback_groups, children)
        };

        let component_type =
            queue
                .hir
                .types
                .create_component_type(component.name, requester, properties, children);

        for (index, property_name, property_type, callbacks) in callback_groups {
            let self_name = queue.hir.intern_name("self");
            let mut implementations = Vec::with_capacity(callbacks.len());
            for callback in callbacks {
                if !callback.generics.type_params.is_empty() {
                    return Err(HIRError::new(
                        requester,
                        crate::HIRErrorKind::NotImplemented(callback.name),
                        callback.span,
                    ));
                }
                let definition = queue
                    .hir
                    .types
                    .callbacks
                    .definition_named(property_type, callback.name)
                    .ok_or_else(|| {
                        HIRError::new(
                            requester,
                            crate::HIRErrorKind::CallbackContractNotFound {
                                callback: callback.name,
                            },
                            callback.span,
                        )
                    })?;
                let actual = callback
                    .args
                    .iter()
                    .filter(|argument| argument.data.name.data != self_name)
                    .count();
                if actual != definition.mapping.len() {
                    return Err(HIRError::new(
                        requester,
                        crate::HIRErrorKind::CallbackArgumentCountMismatch {
                            callback: callback.name,
                            expected: definition.mapping.len(),
                            actual,
                        },
                        callback.span,
                    ));
                }
                let declaration_name = queue.hir.intern_name(&format!(
                    "{}_callback{index}Of{}_{}",
                    queue.hir.get_name(component.name),
                    queue.hir.get_name(property_name),
                    queue.hir.get_name(callback.name),
                ));
                let function = queue.insert_hidden_function(
                    requester,
                    callback,
                    component_type,
                    declaration_name,
                )?;
                let function_view = queue.hir.view(function);
                for (position, expected) in definition.argument_types.iter().enumerate() {
                    let actual = function_view
                        .get_argument_type((position + 1) as u8)
                        .expect("callback implementation must have one argument per mapping");
                    if actual != *expected {
                        return Err(HIRError::unexpected_type(
                            requester,
                            actual,
                            *expected,
                            callback.span,
                        ));
                    }
                }
                implementations.push(PropertyCallback {
                    name: callback.name,
                    function,
                    order: definition.order,
                    mapping: definition.mapping,
                });
            }
            queue.hir.types.callbacks.set_property_callbacks(
                component_type,
                index,
                implementations,
            );
        }

        Ok(component_type)
    }

    /// Resolve a component's signature with cycle detection.
    pub(super) fn resolve_component_signature(
        &self,
        queue: &HirQueueBuilder<'a>,
        node: &HirNode,
        component: &'a ComponentDeclaration,
    ) -> Result<TermId> {
        let key = (node.entry, component.name);

        // Push onto cycle-detection stack
        node.pendings.signature_stack.borrow_mut().push(key);

        // Insert into in-progress set. If already present, we have a cycle.
        if !node.pendings.signatures_in_progress.insert(key) {
            let chain = node.pendings.signature_stack.borrow().clone();
            node.pendings.signature_stack.borrow_mut().pop();
            return Err(HIRError::cyclic_component_signature(
                node.entry,
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
