use common::{Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, builders::HirQueueBuilder,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(super) fn build_function_call(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        name: Spanned<DedupPoolId<Type>>,
        args: &[Spanned<DedupPoolId<ASTExpression>>],
        context: &TypeContext,
    ) -> Result<HirExpression> {
        let identifier = queue.get_plain_type(name);
        let func = self.lookup_function(
            queue,
            name.span.make_spanned(identifier.identifier),
            self.file(),
        )?;
        let func_viewer = queue.hir.view(func);
        let func_ty_view = func_viewer.ty();
        let func_real_type = func_ty_view
            .is_function()
            .expect("Function should have function type");

        let expected_args = func_real_type.arguments();

        if expected_args.len() != args.len() {
            let func_name = identifier.identifier;
            return Err(HIRError::invalid_funcall_arg_length(
                func_name,
                expected_args.len(),
                args.len(),
                name.span,
            ));
        }
        let mut generics = queue
            .get_node(self.file())
            .resolve_call_generics(identifier, context)?;
        let args = args
            .iter()
            .zip(expected_args)
            .map(|(arg, ty)| {
                let expected_ty = match queue.hir.view(*ty).raw() {
                    HirType::GenericParam { index, .. } => generics.get(*index as usize).cloned(),
                    _ => None,
                };

                let expr = self.build_expression(queue, *arg, expected_ty, context)?;
                if let HirType::GenericParam { index, .. } = queue.hir.view(*ty).raw()
                    && let None = expected_ty
                {
                    let expr_ty = queue.hir.view(expr.data).ty();
                    generics.insert(*index as usize, expr_ty);
                }
                Ok(expr)
            })
            .collect::<Result<_>>()?;
        let ty = func_real_type.return_type();
        Ok(HirExpression {
            kind: HirExpressionKind::FunctionCall {
                name: func,
                args,
                generics,
            },
            ty,
        })
    }
}
