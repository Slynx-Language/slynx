use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirExpression, HirExpressionKind, HirFunctionDeclaration, Owned,
    Result, builders::HirQueueBuilder, generics::GenericTypeArguments, term::TermId,
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
    /// Builds the explicit (non-receiver) arguments of an already-resolved
    /// method call. Returns the argument list and the method return type.
    pub(super) fn build_resolved_method_arguments(
        &mut self,
        queue: &HirQueueBuilder<'_>,
        target: DeclarationId<HirFunctionDeclaration>,
        type_arguments: &[Spanned<DedupPoolId<Type>>],
        arguments: &[Spanned<DedupPoolId<ASTExpression>>],
        span: Span,
        context: &TypeContext,
    ) -> Result<(Vec<Spanned<PoolId<HirExpression>>>, TermId)> {
        let func_viewer = queue.hir.view(target);
        let (expected_args, return_type) = func_viewer.type_viewer();
        if expected_args.len() != arguments.len() + 1 {
            return Err(HIRError::invalid_funcall_arg_length(
                self.file(),
                func_viewer.name(),
                expected_args.len(),
                arguments.len() + 1,
                span,
            ));
        }
        let explicit_expected = &expected_args[1..];
        let mut generics = GenericTypeArguments::from_explicit(
            type_arguments
                .iter()
                .map(|ty| {
                    queue
                        .lowerer
                        .lower_type(queue, self.file(), *ty, context)
                        .map(|owned| owned.term)
                })
                .collect::<Result<Vec<_>>>()?,
        );
        let args = arguments
            .iter()
            .zip(explicit_expected)
            .map(|(arg, ty)| {
                let expected_ty = match queue.hir.view(*ty).raw().is_var_type() {
                    Some(var) => generics.get(var.index as usize),
                    None => Some(*ty),
                };
                let expr = self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *arg,
                        expected: expected_ty,
                        context,
                    },
                )?;
                self.unify_terms(queue, queue.hir.view(expr.data).ty().term, *ty, span)?;
                if let Some(var) = queue.hir.view(*ty).raw().is_var_type() {
                    generics.try_set(var.index as usize, queue.hir.view(expr.data).ty().term);
                }
                Ok(expr)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((args, return_type))
    }

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
        let func_owner = func_viewer.data().owner;
        let (expected_args, return_type) = func_viewer.type_viewer();

        if expected_args.len() != total_arguments {
            let name = func_viewer.name();
            return Err(HIRError::invalid_funcall_arg_length(
                self.file(),
                name,
                expected_args.len(),
                total_arguments,
                span,
            ));
        }
        let mut generics = GenericTypeArguments::from_explicit(
            type_arguments
                .iter()
                .map(|ty| {
                    queue
                        .lowerer
                        .lower_type(queue, self.file(), *ty, context)
                        .map(|owned| owned.term)
                })
                .collect::<Result<Vec<_>>>()?,
        );
        let args = {
            // A method's signature includes the receiver as its first argument,
            // while `arguments` only holds the explicit call arguments. The
            // receiver lives in `prepended_arguments`, so the parameters have to
            // be split at the same position to pair every argument with the
            // parameter it actually corresponds to.
            let (prepended_expected, explicit_expected) =
                expected_args.split_at(prepended_arguments.len());

            for (arg, ty) in prepended_arguments.iter().zip(prepended_expected) {
                let expr_ty = queue.hir.view(arg.data).ty();
                self.unify_terms(queue, expr_ty.term, *ty, span)?;
            }

            let transformed_arguments = arguments
                .iter()
                .zip(explicit_expected)
                .map(|(arg, ty)| {
                    let expected_ty = match queue.hir.view(*ty).raw().is_var_type() {
                        Some(var) => generics.get(var.index as usize),
                        None => Some(*ty),
                    };

                    let expr = self.build_expression(
                        queue,
                        ExpressionDescriptor {
                            target: *arg,
                            expected: expected_ty,
                            context,
                        },
                    )?;
                    {
                        let expr_ty = queue.hir.view(expr.data).ty();
                        self.unify_terms(queue, expr_ty.term, *ty, span)?;
                    }
                    if let Some(var) = queue.hir.view(*ty).raw().is_var_type() {
                        generics.try_set(var.index as usize, queue.hir.view(expr.data).ty().term);
                    }
                    Ok(expr)
                })
                .collect::<Result<Vec<_>>>()?;
            prepended_arguments
                .iter()
                .cloned()
                .chain(transformed_arguments)
                .collect()
        };

        Ok(HirExpression {
            kind: HirExpressionKind::FunctionCall {
                name: target,
                args,
                generics: generics.into_vec(),
            },
            ty: Owned::new(func_owner, return_type),
        })
    }
}
