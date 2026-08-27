use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirExpression, HirExpressionKind, HirFunctionDeclaration, HirType,
    Result, builders::HirQueueBuilder,
};

use super::ExpressionBuilder;

///A descriptor for a function call expression.
pub struct FunctionCallDescriptor<'a> {
    ///The function that is being called
    pub target: DeclarationId<HirFunctionDeclaration>,
    ///The arguments passed to this function call
    pub received_args: &'a [Spanned<DedupPoolId<ASTExpression>>],
    ///The generic arguments passed to this function call
    pub received_generics: &'a [Spanned<DedupPoolId<Type>>],
    ///The type context for this function call
    pub context: &'a TypeContext<'a>,
}

impl ExpressionBuilder {
    pub(crate) fn build_call_for(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        descriptor: Spanned<FunctionCallDescriptor>,
    ) -> Result<HirExpression> {
        let func_viewer = queue.hir.view(descriptor.target);
        let type_viewer = func_viewer.type_viewer();

        let expected_args = type_viewer.arguments();

        if expected_args.len() != descriptor.received_args.len() {
            let name = func_viewer.name();
            return Err(HIRError::invalid_funcall_arg_length(
                name,
                expected_args.len(),
                descriptor.received_args.len(),
                descriptor.span,
            ));
        }
        let mut generics = queue
            .get_node(self.file())
            .resolve_call_generics(descriptor.received_generics, descriptor.context)?;
        let args = {
            descriptor
                .received_args
                .iter()
                .zip(expected_args)
                .map(|(arg, ty)| {
                    let expected_ty = match queue.hir.view(*ty).raw() {
                        HirType::GenericParam { index, .. } => {
                            generics.get(*index as usize).cloned()
                        }
                        _ => None,
                    };

                    let expr =
                        self.build_expression(queue, *arg, expected_ty, descriptor.context)?;
                    if let HirType::GenericParam { index, .. } = queue.hir.view(*ty).raw()
                        && let None = expected_ty
                    {
                        let expr_ty = queue.hir.view(expr.data).ty();
                        generics.insert(*index as usize, expr_ty);
                    }
                    Ok(expr)
                })
                .collect::<Result<_>>()?
        };
        let ty = type_viewer.return_type();
        Ok(HirExpression {
            kind: HirExpressionKind::FunctionCall {
                name: descriptor.target,
                args,
                generics,
            },
            ty,
        })
    }
    /// Builds a function call expression for the function with the given `name`and `args`. This function might be changed to `build_call_for`
    pub(super) fn build_function_call(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        name: Spanned<DedupPoolId<Type>>,
        args: &[Spanned<DedupPoolId<ASTExpression>>],
        context: &TypeContext,
        span: Span,
    ) -> Result<HirExpression> {
        let identifier = queue.get_plain_type(name);
        let func = self.lookup_function(
            queue,
            name.span.make_spanned(identifier.identifier),
            self.file(),
        )?;

        let descriptor = FunctionCallDescriptor {
            target: func,
            received_args: args,
            received_generics: &identifier.generic,
            context,
        };
        self.build_call_for(queue, span.make_spanned(descriptor))
    }
}
