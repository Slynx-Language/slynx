use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use module_loader::ASTType;
use slynx_parser::{ASTExpression, Type};

use crate::{
    DeclarationId, DescriptorId, ExpressionBuilder, ExpressionDescriptor, HIRError, HirExpression,
    HirExtendDeclaration, HirFunctionDeclaration, HirQueueBuilder, InterfaceType, Owned, Result,
    SymbolPointer,
    enums::{EnumExpressionDescriptor, EnumVariantDescriptor},
    field_access::FieldAccessDescriptor,
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
                if let Some(method) =
                    queue.resolve_method(ty.owner, ty.term, queue.type_name(**name), span)? =>
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
}
