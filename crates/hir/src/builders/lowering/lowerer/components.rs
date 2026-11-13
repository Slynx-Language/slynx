use std::collections::HashMap;

use common::{Spanned, pool::DedupPoolId};
use slynx_parser::{
    ASTExpression, ComponentDeclaration, ComponentMemberKind, FuncDeclaration, TypeContext,
};

use crate::{
    ComponentChild, HIRError, HirNode, HirQueueBuilder, PropertyCallback, Result, SymbolPointer,
    arrays::ArrayTerm,
    builders::lowering::ASTLowerer,
    term::{Term, TermId},
    vector::VectorTerm,
};

#[derive(Clone, Copy)]
enum MemberInfo {
    /// Input or property with a known type.
    Value(TermId),
    /// View with RHS expression to infer from.
    View(Spanned<DedupPoolId<ASTExpression>>),
    /// Named child component instance with known component type.
    Child(TermId),
}

struct ViewInferCtx<'q, 'a, 'm> {
    queue: &'q HirQueueBuilder<'a>,
    requester: module_loader::FileId,
    members: &'m HashMap<SymbolPointer, MemberInfo>,
    cache: &'m mut HashMap<SymbolPointer, TermId>,
    computing: &'m mut Vec<SymbolPointer>,
}

impl<'q, 'a, 'm> ViewInferCtx<'q, 'a, 'm> {
    fn infer(&mut self, expr: Spanned<DedupPoolId<ASTExpression>>) -> Result<(TermId, bool)> {
        use ASTExpression::*;
        let ast = self.queue.modules.get_expr(expr.data).clone();
        match ast {
            IntLiteral(_) => Ok((
                self.queue
                    .hir
                    .types
                    .create_type(Term::signed_integer_type(32)),
                false,
            )),
            FloatLiteral(_) => Ok((
                self.queue.hir.types.create_type(Term::float32_type()),
                false,
            )),
            StringLiteral(_) => Ok((self.queue.hir.types.create_type(Term::string_type()), false)),
            True | False => Ok((
                self.queue.hir.types.create_type(Term::boolean_type()),
                false,
            )),
            Identifier(name) => self.infer_identifier(name, expr.span),
            Tuple(elements) => {
                let mut fields = Vec::with_capacity(elements.len());
                let mut dep = false;
                for element in elements {
                    let (ty, d) = self.infer(element)?;
                    fields.push(ty);
                    dep |= d;
                }
                Ok((self.queue.hir.types.create_tuple_type(fields), dep))
            }
            TupleAccess { tuple, index } => {
                let (tuple_ty, dep) = self.infer(tuple)?;
                let view = self.queue.hir.view(tuple_ty).nominal();
                let fields = view
                    .is_tuple()
                    .ok_or_else(|| HIRError::couldnt_infer(self.requester, expr.span))?;
                let ty = *fields
                    .get(index as usize)
                    .ok_or_else(|| HIRError::couldnt_infer(self.requester, expr.span))?;
                Ok((ty, dep))
            }
            Binary { lhs, op, rhs } => {
                let (lhs_ty, lhs_dep) = self.infer(lhs)?;
                let (_rhs_ty, rhs_dep) = self.infer(rhs)?;
                let dep = lhs_dep || rhs_dep;
                let ty = if op.is_logical() {
                    self.queue.hir.types.create_type(Term::boolean_type())
                } else {
                    lhs_ty
                };
                Ok((ty, dep))
            }
            Negate(inner) => self.infer(inner),
            Reference {
                mutable,
                expr: inner,
            } => {
                let (inner_ty, dep) = self.infer(inner)?;
                let ref_ty = if mutable {
                    Term::mutable_reference(inner_ty)
                } else {
                    Term::reference(inner_ty)
                };
                Ok((self.queue.hir.types.create_type(ref_ty), dep))
            }
            Deref(inner) => {
                let (inner_ty, dep) = self.infer(inner)?;
                let view = self.queue.hir.view(inner_ty);
                let ty = if let Some(t) = view.is_mutable_ref() {
                    t
                } else if let Some(t) = view.is_imutable_ref() {
                    t
                } else {
                    inner_ty
                };
                Ok((ty, dep))
            }
            FieldAccess { parent, field } => {
                let (parent_ty, dep) = self.infer(parent)?;
                let field_ast = self.queue.modules.get_expr(field.data).clone();
                let ty = match field_ast {
                    Identifier(field_name) => {
                        self.infer_named_field(parent_ty, field_name, expr.span)?
                    }
                    IntLiteral(idx) if idx >= 0 => {
                        self.infer_tuple_index(parent_ty, idx as usize, expr.span)?
                    }
                    _ => return Err(HIRError::couldnt_infer(self.requester, expr.span)),
                };
                Ok((ty, dep))
            }
            Array(elements) => {
                if let Some(first) = elements.first().copied() {
                    let (elem_ty, dep) = self.infer(first)?;
                    let array_ext = self
                        .queue
                        .hir
                        .types
                        .create_type(Term::extension_type(ArrayTerm));
                    let len = self
                        .queue
                        .hir
                        .types
                        .create_type(Term::const_usize_type(elements.len()));
                    let ty = self
                        .queue
                        .hir
                        .types
                        .create_type(Term::application(array_ext, vec![elem_ty, len]));
                    Ok((ty, dep))
                } else {
                    Err(HIRError::couldnt_infer(self.requester, expr.span))
                }
            }
            Vector(elements) => {
                if let Some(first) = elements.first().copied() {
                    let (elem_ty, dep) = self.infer(first)?;
                    let vec_ext = self
                        .queue
                        .hir
                        .types
                        .create_type(Term::extension_type(VectorTerm));
                    let ty = self
                        .queue
                        .hir
                        .types
                        .create_type(Term::application(vec_ext, vec![elem_ty]));
                    Ok((ty, dep))
                } else {
                    Err(HIRError::couldnt_infer(self.requester, expr.span))
                }
            }
            IndexExpression(target, _) => {
                let (target_ty, dep) = self.infer(target)?;
                let view = self.queue.hir.view(target_ty).nominal();
                if let Some((elem, _len)) = view.is_array() {
                    Ok((elem, dep))
                } else if let Some(elem) = view.is_vector() {
                    Ok((elem, dep))
                } else {
                    Err(HIRError::couldnt_infer(self.requester, expr.span))
                }
            }
            Matches { lhs, .. } => {
                let (_lhs_ty, dep) = self.infer(lhs)?;
                Ok((self.queue.hir.types.create_type(Term::boolean_type()), dep))
            }
            If { .. } | FunctionCall { .. } | ObjectExpression { .. } | Component(_) => {
                Err(HIRError::couldnt_infer(self.requester, expr.span))
            }
        }
    }

    fn infer_identifier(
        &mut self,
        name: SymbolPointer,
        span: common::Span,
    ) -> Result<(TermId, bool)> {
        let Some(info) = self.members.get(&name).copied() else {
            // Unknown identifier in view context: cannot infer and does not satisfy dependency.
            return Err(HIRError::couldnt_infer(self.requester, span));
        };
        match info {
            MemberInfo::Value(ty) | MemberInfo::Child(ty) => Ok((ty, true)),
            MemberInfo::View(rhs) => {
                if let Some(&ty) = self.cache.get(&name) {
                    return Ok((ty, true));
                }
                if self.computing.contains(&name) {
                    return Err(HIRError::couldnt_infer(self.requester, rhs.span));
                }
                self.computing.push(name);
                let (ty, dep) = self.infer(rhs)?;
                self.computing.pop();
                if !dep {
                    return Err(HIRError::view_missing_dependencies(
                        self.requester,
                        name,
                        rhs.span,
                    ));
                }
                self.cache.insert(name, ty);
                Ok((ty, true))
            }
        }
    }

    fn infer_named_field(
        &self,
        parent_ty: TermId,
        field_name: SymbolPointer,
        span: common::Span,
    ) -> Result<TermId> {
        let view = self.queue.hir.view(parent_ty).nominal();
        if let Some(component) = view.is_component() {
            if let Some(prop) = component.props().iter().find(|p| p.data.name == field_name) {
                return Ok(prop.data.ty);
            }
        } else if let Some(strukt) = view.is_struct() {
            if let Some(field) = strukt.fields().iter().find(|f| f.data.name == field_name) {
                return Ok(field.data.ty);
            }
        }
        Err(HIRError::couldnt_infer(self.requester, span))
    }

    fn infer_tuple_index(
        &self,
        parent_ty: TermId,
        index: usize,
        span: common::Span,
    ) -> Result<TermId> {
        let view = self.queue.hir.view(parent_ty).nominal();
        let fields = view
            .is_tuple()
            .ok_or_else(|| HIRError::couldnt_infer(self.requester, span))?;
        fields
            .get(index)
            .copied()
            .ok_or_else(|| HIRError::couldnt_infer(self.requester, span))
    }
}

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

        // First pass: collect member info for view inference and known child types.
        let mut members: HashMap<SymbolPointer, MemberInfo> = HashMap::new();
        for member in &component.members {
            match &member.kind {
                ComponentMemberKind::Input { name, ty } => {
                    let input = self.lower_type(queue, requester, *ty, &context)?;
                    members.insert(*name, MemberInfo::Value(input.term));
                }
                ComponentMemberKind::Property { name, ty, .. } => {
                    let Some(ty) = ty else {
                        return Err(HIRError::component_missing_prop_type(
                            requester,
                            member.span,
                        ));
                    };
                    let field = self.lower_type(queue, requester, *ty, &context)?.term;
                    members.insert(*name, MemberInfo::Value(field));
                }
                ComponentMemberKind::View { name, rhs, .. } => {
                    members.insert(*name, MemberInfo::View(*rhs));
                }
                ComponentMemberKind::NamedChild { name, child } => {
                    let ty = self
                        .lower_type(queue, requester, child.data.name, &context)?
                        .term;
                    if queue.hir.view(ty).nominal().is_component().is_none() {
                        return Err(HIRError::not_a_component(requester, ty, child.span));
                    }
                    members.insert(*name, MemberInfo::Child(ty));
                }
                ComponentMemberKind::Child(_) => {}
            }
        }

        let mut view_cache: HashMap<SymbolPointer, TermId> = HashMap::new();

        let (callback_groups, children) = {
            let mut children = Vec::with_capacity(component.members.len());
            let mut callback_groups: Vec<(usize, SymbolPointer, TermId, Vec<&FuncDeclaration>)> =
                Vec::new();
            let mut property_index = 0;
            for member in &component.members {
                match &member.kind {
                    ComponentMemberKind::Input { name, .. } => {
                        let ty = match members.get(name) {
                            Some(MemberInfo::Value(ty)) => *ty,
                            _ => unreachable!("input type collected in first pass"),
                        };
                        properties.push((*name, ty));
                    }
                    ComponentMemberKind::View { name, rhs, .. } => {
                        let ty = if let Some(&cached) = view_cache.get(name) {
                            cached
                        } else {
                            let mut computing = Vec::new();
                            let (ty, dep) = {
                                let mut ctx = ViewInferCtx {
                                    queue,
                                    requester,
                                    members: &members,
                                    cache: &mut view_cache,
                                    computing: &mut computing,
                                };
                                ctx.infer(*rhs)?
                            };
                            if !dep {
                                return Err(HIRError::view_missing_dependencies(
                                    requester, *name, rhs.span,
                                ));
                            }
                            view_cache.insert(*name, ty);
                            ty
                        };
                        properties.push((*name, ty));
                    }
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
                        let ty = match members.get(name) {
                            Some(MemberInfo::Child(ty)) => *ty,
                            _ => {
                                let ty = self
                                    .lower_type(queue, requester, c.data.name, &context)?
                                    .term;
                                ty
                            }
                        };
                        if queue.hir.view(ty).nominal().is_component().is_some() {
                            children.push(ComponentChild {
                                name: Some(*name),
                                ty,
                            });
                        } else {
                            return Err(HIRError::not_a_component(requester, ty, c.span));
                        }
                    }
                    ComponentMemberKind::Child(c) => {
                        let ty = self
                            .lower_type(queue, requester, c.data.name, &context)?
                            .term;
                        if queue.hir.view(ty).nominal().is_component().is_some() {
                            children.push(ComponentChild { name: None, ty });
                        } else {
                            return Err(HIRError::not_a_component(requester, ty, c.span));
                        }
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
