use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirExpression, HirExpressionKind, HirFunctionDeclaration, HirType,
    Result, builders::HirQueueBuilder,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

///How the target of a function call is resolved.
pub enum FunctionTarget<'a> {
    ///The function has already been resolved to a declaration, along with any
    ///explicit generic type arguments from the call site. Used for method
    ///calls, where the target is resolved against its receiver type.
    Resolved {
        ///The function that is being called
        target: DeclarationId<HirFunctionDeclaration>,
        ///The explicit generic type arguments passed to this call, if any
        type_arguments: &'a [Spanned<DedupPoolId<Type>>],
    },
    ///The function is referenced by its name in source (`foo<T>(a, b)`).
    Free {
        ///The type of the call, from which the function name and its generic
        ///type arguments are derived
        name: Spanned<DedupPoolId<Type>>,
    },
}

///A descriptor for a function call expression.
pub struct FunctionCallDescriptor<'a> {
    ///How the function being called is resolved
    pub target: FunctionTarget<'a>,
    ///The arguments passed to this function call
    pub arguments: &'a [Spanned<DedupPoolId<ASTExpression>>],
    ///The arguments that are prepended to the call. This is useful method calls for example where the function call is such as 'a.call(5)', but the desugaring would transform it into 'call(a,5)'
    pub prepended_arguments: &'a [Spanned<PoolId<HirExpression>>],
    ///The span of the call expression, used for error reporting
    pub span: Span,
    ///The type context for this function call
    pub context: &'a TypeContext<'a>,
}

impl FunctionCallDescriptor<'_> {
    ///Returns the total number of arguments passed to this function call, including prepended arguments.
    pub fn total_argument_length(&self) -> usize {
        self.arguments.len() + self.prepended_arguments.len()
    }
}

impl ExpressionBuilder {
    /// Builds a function call expression from a descriptor. This is the single
    /// entry point for building function calls: it resolves the target when
    /// given by name, checks the argument count, resolves the explicit generic
    /// type arguments, infers any that are missing from the arguments, and
    /// prepends arguments such as the receiver of a method call.
    pub(super) fn build_function_call(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        descriptor: FunctionCallDescriptor<'_>,
    ) -> Result<HirExpression> {
        let total_arguments = descriptor.total_argument_length();
        let FunctionCallDescriptor {
            target,
            arguments,
            prepended_arguments,
            span,
            context,
        } = descriptor;

        let (target, type_arguments) = match target {
            FunctionTarget::Resolved {
                target,
                type_arguments,
            } => (target, type_arguments),
            FunctionTarget::Free { name } => {
                let identifier = queue.get_plain_type(name);
                let target = self.lookup_function(
                    queue,
                    name.span.make_spanned(identifier.identifier),
                    self.file(),
                )?;
                (target, &identifier.generic[..])
            }
        };

        let func_viewer = queue.hir.view(target);
        let type_viewer = func_viewer.type_viewer();

        let expected_args = type_viewer.arguments();

        if expected_args.len() != total_arguments {
            let name = func_viewer.name();
            return Err(HIRError::invalid_funcall_arg_length(
                name,
                expected_args.len(),
                total_arguments,
                span,
            ));
        }
        let mut generics = queue
            .get_node(self.file())
            .resolve_call_generics(type_arguments, context)?;
        let args = {
            let transformed_arguments = arguments
                .iter()
                .zip(expected_args)
                .map(|(arg, ty)| {
                    let expected_ty = match queue.hir.view(*ty).raw() {
                        HirType::GenericParam { index, .. } => {
                            generics.get(*index as usize).cloned()
                        }
                        _ => None,
                    };

                    let expr = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: *arg,
                            expected: expected_ty,
                            context,
                        },
                    )?;
                    if let HirType::GenericParam { index, .. } = queue.hir.view(*ty).raw()
                        && let None = expected_ty
                    {
                        let expr_ty = queue.hir.view(expr.data).ty();
                        generics.insert(*index as usize, expr_ty);
                    }
                    Ok(expr)
                })
                .collect::<Result<Vec<_>>>()?;
            prepended_arguments
                .iter()
                .cloned()
                .chain(transformed_arguments.into_iter())
                .collect()
        };
        let ty = type_viewer.return_type();
        Ok(HirExpression {
            kind: HirExpressionKind::FunctionCall {
                name: target,
                args,
                generics,
            },
            ty,
        })
    }
}
