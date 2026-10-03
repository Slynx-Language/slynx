use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::ASTType;
use slynx_parser::{ASTExpression, Type};

use crate::{
    DeclarationId, DescriptorId, ExpressionBuilder, ExpressionDescriptor, HIRError, HirExpression,
    HirExtendDeclaration, HirFunctionDeclaration, HirQueueBuilder, InterfaceType, Owned, Result,
    SymbolPointer,
    field_access::FieldAccessDescriptor,
    resolution::types::{
        FindInherentMethodDescriptor, FindInterfaceMethodDescriptor, TypeMethodResolution,
    },
    term::{TermId, TermNode},
};

pub enum MethodSource {
    Inherent,
    Interface {
        id: DedupPoolId<InterfaceType>,
        implementor: DedupPoolId<HirExtendDeclaration>,
    },
}
pub struct ResolvedMethod {
    pub target: DeclarationId<HirFunctionDeclaration>,
    pub source: MethodSource,
}

pub enum AccessParentCategory {
    TypeAccess {
        ast_type: ASTType,
        span: Span,
    },
    ///Represents an normal access field, where the given expression is the parent
    NormalAccess(Spanned<PoolId<HirExpression>>),
}

pub enum TypeAccessCategory {
    ///Represents an enum variant access. More specifically for some that does not contain a payload.
    EnumVariant(usize),
    EnumPayloadVariant {
        ///Since this is the same as a function call, this can include generics and is passed the type name instead
        payload_name: Spanned<DedupPoolId<Type>>,
        variant_id: usize,
        args: Vec<Spanned<DedupPoolId<ASTExpression>>>,
    },
    StaticMethod {
        target: DeclarationId<HirFunctionDeclaration>,
        name: Spanned<DedupPoolId<Type>>,
        args: Vec<Spanned<DedupPoolId<ASTExpression>>>,
    },
    Intermediate {
        inner_parent: Spanned<DedupPoolId<ASTExpression>>,
        inner_field: Spanned<DedupPoolId<ASTExpression>>,
    },
}

pub enum FieldAccessCategory {
    Field(SymbolPointer),
    Intermediate {
        inner_parent: Spanned<DedupPoolId<ASTExpression>>,
        inner_field: Spanned<DedupPoolId<ASTExpression>>,
    },
    Method {
        target: DeclarationId<HirFunctionDeclaration>,
        name: Spanned<DedupPoolId<Type>>,
        args: Vec<Spanned<DedupPoolId<ASTExpression>>>,
    },
}

impl ExpressionBuilder {
    pub fn resolve_access_field_category(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: &FieldAccessDescriptor,
    ) -> Result<AccessParentCategory> {
        match &descriptor.parent {
            Either::Left(parent)
                if let ASTExpression::Identifier(ident) = queue.get_expr(parent.data)
                    && let Some(ast_type) = queue.lowerer.lookup.find_type(self.file(), *ident) =>
            {
                Ok(AccessParentCategory::TypeAccess {
                    ast_type,
                    span: parent.span,
                })
            }
            Either::Left(parent) => {
                let parent = self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *parent,
                        expected: descriptor.expected,
                        context: descriptor.context,
                    },
                )?;
                Ok(AccessParentCategory::NormalAccess(parent))
            }

            Either::Right(parent) => Ok(AccessParentCategory::NormalAccess(*parent)),
        }
    }

    ///Finds the category of the type access. The given `child` is the child expression being checked, and the given `ty` is the type being accessed. For example Struct.thing() this represents `ty` = ´Struct`, and `child` = `thing()`
    pub fn resolve_type_access_category(
        &mut self,
        queue: &HirQueueBuilder,
        child: Spanned<DedupPoolId<ASTExpression>>,
        ty: Owned<TermId>,
        span: Span,
    ) -> Result<TypeAccessCategory> {
        let node = queue.hir.view(ty.term).dereference().raw().node();
        match queue.get_expr(child.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => Ok(TypeAccessCategory::Intermediate {
                inner_parent: *inner_parent,
                inner_field: *inner_field,
            }),
            ASTExpression::Identifier(name)
                if let TermNode::Data(DescriptorId::Enum(e)) = node
                    && let Some(variant_id) = queue.hir.view(*e).find_variant(*name) =>
            {
                let enum_viewer = queue.hir.view(*e);
                let raw_variant = &enum_viewer.variants()[variant_id];
                if raw_variant.payload.is_empty() {
                    Ok(TypeAccessCategory::EnumVariant(variant_id))
                } else {
                    return Err(HIRError::invalid_funcall_arg_length(
                        *name,
                        raw_variant.payload.len(),
                        0,
                        span,
                    ));
                }
            }
            ASTExpression::Identifier(_) => {
                unimplemented!("Constant values bound to types are not supported yet")
            }

            ASTExpression::FunctionCall {
                name,
                args: arguments,
            } if let TermNode::Data(DescriptorId::Enum(e)) = node
                && let Some((id, variant_type)) = queue
                    .hir
                    .view(*e)
                    .variants()
                    .iter()
                    .enumerate()
                    .find(|(_, variant)| variant.name == queue.type_name(name.data))
                && !variant_type.payload.is_empty() =>
            {
                Ok(TypeAccessCategory::EnumPayloadVariant {
                    variant_id: id,
                    payload_name: *name,
                    args: arguments.to_vec(),
                })
            }

            ASTExpression::FunctionCall { name, args }
                if let Some(method) = self.find_inherent_method_of(
                    queue,
                    FindInherentMethodDescriptor {
                        ty,
                        name: queue.type_name(**name),
                        span,
                    },
                )? =>
            {
                let name = *name;
                Ok(TypeAccessCategory::StaticMethod {
                    name,
                    target: method,
                    args: args.to_vec(),
                })
            }

            _ => Err(HIRError::invalid_type_access(span)),
        }
    }

    pub fn resolve_field_access_category(
        &mut self,
        queue: &HirQueueBuilder,
        parent: Spanned<PoolId<HirExpression>>,
        child: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
    ) -> Result<FieldAccessCategory> {
        let category = match queue.get_expr(child.data) {
            ASTExpression::FieldAccess {
                parent: inner_parent,
                field: inner_field,
            } => FieldAccessCategory::Intermediate {
                inner_parent: *inner_parent,
                inner_field: *inner_field,
            },
            ASTExpression::Identifier(field_name) => FieldAccessCategory::Field(*field_name),
            ASTExpression::FunctionCall { name, args } => {
                let name_sym = queue.type_name(name.data);
                let parent_type_view = queue.hir.view(queue.hir[parent.data].ty);
                let parent_ty = parent_type_view.dereference();

                let inherent = match parent_ty.is_struct() {
                    Some(view) => view
                        .method_named_as(name_sym, VisibilityModifier::Public)
                        .or_else(|| queue.hir.types.methods.method_of(parent_ty.data, name_sym)),
                    None => None,
                };
                let func_id = match inherent {
                    Some(id) => id,
                    None if let Some(id) = self.find_inherent_method_of(
                        queue,
                        FindInherentMethodDescriptor {
                            ty: Owned {
                                owner: self.file(),
                                term: parent_ty.data,
                            },
                            name: name_sym,
                            span,
                        },
                    )? =>
                    {
                        id
                    }
                    None => {
                        let concrete_self = {
                            let mut current = queue.hir[parent.data].ty;
                            loop {
                                match queue.hir.view(current).raw().node() {
                                    TermNode::Ref { target, .. } => current = *target,
                                    _ => break current,
                                }
                            }
                        };
                        match self.find_interface_method_of(
                            queue,
                            FindInterfaceMethodDescriptor {
                                ty: Owned {
                                    owner: self.file(),
                                    term: concrete_self,
                                },
                                name: name_sym,
                                span,
                            },
                        )? {
                            Some(TypeMethodResolution::Interface { method, .. }) => method,
                            Some(TypeMethodResolution::Inherent(method))
                            | Some(TypeMethodResolution::Static(method)) => method,
                            None => {
                                return Err(HIRError::missing_properties(vec![name_sym], span));
                            }
                        }
                    }
                };

                FieldAccessCategory::Method {
                    target: func_id,
                    name: *name,
                    args: args.to_vec(),
                }
            }
            _ => return Err(HIRError::invalid_field_access(span)),
        };
        Ok(category)
    }
}
