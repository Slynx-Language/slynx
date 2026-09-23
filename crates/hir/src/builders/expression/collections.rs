use common::{Span, Spanned, pool::DedupPoolId};
use slynx_parser::{ASTExpression, RangeType, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, Result,
    arrays::ArrayTerm,
    builders::HirQueueBuilder,
    term::{PrimitiveType, Term, TermId, TermNode},
    vector::VectorTerm,
};

use super::{ExpressionBuilder, ExpressionDescriptor};

///Which collection literal kind a [`build_sequence`](ExpressionBuilder::build_sequence)
///call produces. Arrays may carry a fixed length from the expected type;
///vectors always infer their element type.
#[derive(Clone, Copy)]
pub enum SequenceKind {
    Array,
    Vector,
}
///A descriptor used to generate tuple expressions. Such as `(1,"thing", variable)`
pub struct TupleExpressionDescriptor<'a> {
    pub context: &'a TypeContext<'a>,
    pub expected: Option<TermId>,
    ///The fields expressions on the tuple
    pub fields: &'a [Spanned<DedupPoolId<ASTExpression>>],
}

///A descriptor used to generate tuple access expressions, such as `var.0`
pub struct TupleAccessDescriptor<'a> {
    ///The tuple expression
    pub tuple: Spanned<DedupPoolId<ASTExpression>>,
    pub expected: Option<TermId>,
    pub span: Span,
    ///The index being accessed on the given tuple expression
    pub index: usize,
    pub context: &'a TypeContext<'a>,
}

///A descriptor used to generate expressions for things such as `array[44 + variable]`
pub struct IndexExpressionDescriptor<'a> {
    ///The expression we are indexing
    pub expr: Spanned<DedupPoolId<ASTExpression>>,
    pub range: &'a RangeType,
    pub expected: Option<TermId>,
    pub span: Span,
    pub context: &'a TypeContext<'a>,
}

///A descriptor used to generate either vector or array expressions
pub struct SequenceExpressionDescriptor<'a> {
    ///The initial values inside the sequence
    pub expressions: &'a [Spanned<DedupPoolId<ASTExpression>>],
    pub span: Span,
    pub expected: Option<TermId>,
    pub context: &'a TypeContext<'a>,
    ///If the sequence is either array or vector
    pub kind: SequenceKind,
}

impl ExpressionBuilder {
    pub(super) fn build_tuple_expression(
        &mut self,
        queue: &HirQueueBuilder,
        TupleExpressionDescriptor {
            context,
            expected,
            fields,
        }: TupleExpressionDescriptor,
    ) -> Result<HirExpression> {
        let mut expressions = Vec::with_capacity(fields.len());
        let mut types = Vec::with_capacity(fields.len());

        for (idx, field) in fields.iter().enumerate() {
            let field_type = if let Some(expected) = expected
                && let Some(fields) = queue.hir.view(expected).is_tuple()
            {
                Some(fields[idx])
            } else {
                None
            };
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: *field,
                    expected: field_type,
                    context,
                },
            )?;
            types.push(queue.hir[expr.data].ty);
            expressions.push(expr);
        }
        Ok(HirExpression {
            ty: queue.hir.types.create_tuple_type(types),
            kind: HirExpressionKind::Tuple(expressions),
        })
    }

    pub(super) fn build_tuple_access(
        &mut self,
        queue: &HirQueueBuilder,
        TupleAccessDescriptor {
            tuple,
            expected,
            span,
            index,
            context,
        }: TupleAccessDescriptor,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: tuple,
                expected,
                context,
            },
        )?;
        let raw_expr = &queue.hir[expr.data];
        let parent_view = queue.hir.view(raw_expr.ty);
        let resolved = parent_view.dereference();
        let ty = match resolved.is_tuple() {
            None => {
                let ty = resolved.data;
                return Err(HIRError::not_a_tuple(ty, span));
            }
            Some(fields) => {
                let field_index = index;
                if field_index >= fields.len() {
                    return Err(HIRError::invalid_tuple_index(
                        field_index,
                        fields.len(),
                        span,
                    ));
                }
                fields[field_index]
            }
        };
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::FieldAccess {
                expr,
                field_index: index,
                field_name: None,
            },
        })
    }

    pub(super) fn build_index(
        &mut self,
        queue: &HirQueueBuilder,
        IndexExpressionDescriptor {
            expr,
            range,
            expected,
            span,
            context,
        }: IndexExpressionDescriptor,
    ) -> Result<HirExpression> {
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: expr,
                expected,
                context,
            },
        )?;
        let after_index_type = {
            let expr_type = queue.hir[expr.data].ty;
            match &queue.hir.types[expr_type].node() {
                TermNode::Var(_) => expr_type,
                TermNode::Apply { target, args }
                    if let TermNode::Extension(e) = queue.hir.types[*target].node()
                        && (e.dyn_eq(&ArrayTerm) || e.dyn_eq(&VectorTerm)) =>
                {
                    args[0]
                }

                _ => return Err(HIRError::invalid_indexing(expr_type, span)),
            }
        };
        match range {
            RangeType::NoRange(index) => {
                let index = self.build_expression(
                    queue,
                    ExpressionDescriptor {
                        target: *index,
                        expected,
                        context,
                    },
                )?;
                let viewer = queue.hir.view(index.data);
                let ty_viewer = viewer.ty_viewer();
                match ty_viewer.raw().node() {
                    TermNode::Primitive(PrimitiveType::Signed { .. })
                    | TermNode::Primitive(PrimitiveType::Unsigned { .. }) => {}
                    _ => {
                        return Err(HIRError::unexpected_type(
                            ty_viewer.data,
                            queue.hir.types.create_type(Term::unsigned_integer_type(32)),
                            index.span,
                        ));
                    }
                }
                Ok(HirExpression {
                    kind: HirExpressionKind::ArrayIndex(expr, index),
                    ty: after_index_type,
                })
            }
            r => unimplemented!("Ranges {r:?} are not implemented yet"),
        }
    }

    pub(super) fn build_sequence(
        &mut self,
        queue: &HirQueueBuilder,
        SequenceExpressionDescriptor {
            expressions,
            span,
            expected,
            context,
            kind,
        }: SequenceExpressionDescriptor,
    ) -> Result<HirExpression> {
        let mut exprs = Vec::with_capacity(expressions.len());
        let Some(first) = expressions.first() else {
            let matches_expected = |ty: TermId| {
                let viewer = queue.hir.view(ty);
                match kind {
                    SequenceKind::Array => viewer.is_array().is_some(),
                    SequenceKind::Vector => viewer.is_vector().is_some(),
                }
            };
            return match expected {
                Some(ty) if matches_expected(ty) => Ok(HirExpression {
                    ty,
                    kind: match kind {
                        SequenceKind::Array => HirExpressionKind::Array(exprs),
                        SequenceKind::Vector => HirExpressionKind::Vector(exprs),
                    },
                }),
                Some(_) | None => Err(HIRError::couldnt_infer(span)),
            };
        };
        let (inner_type, expected_len) = match kind {
            SequenceKind::Vector => (expected.and_then(|e| queue.hir.view(e).is_vector()), None),
            SequenceKind::Array => expected
                .and_then(|e| queue.hir.view(e).is_array())
                .map(|(inner, size)| (Some(inner), Some(size)))
                .unwrap_or((None, None)),
        };
        let expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: *first,
                expected: inner_type,
                context,
            },
        )?;
        let ty = queue.hir[expr.data].ty;
        if let Some(expected) = inner_type {
            self.unify_terms(queue, ty, expected, span)?;
        }
        exprs.push(expr);
        for expr in &expressions[1..] {
            let expr = self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: *expr,
                    expected: Some(ty),
                    context,
                },
            )?;
            exprs.push(expr);
        }
        let final_type = match kind {
            SequenceKind::Vector => queue.hir.types.create_type({
                let ext = queue
                    .hir
                    .types
                    .create_type(Term::extension_type(VectorTerm));
                Term::application(ext, vec![ty])
            }),
            SequenceKind::Array => {
                let final_length = expected_len.unwrap_or(exprs.len());
                if let Some(expected_len) = expected_len
                    && final_length != expected_len
                {
                    return Err(HIRError::array_length_mismatch(
                        expected_len,
                        final_length,
                        span,
                    ));
                }

                queue.hir.types.create_type({
                    let len = queue
                        .hir
                        .types
                        .create_type(Term::const_usize_type(final_length));
                    let ext = queue.hir.types.create_type(Term::extension_type(ArrayTerm));
                    Term::application(ext, vec![ty, len])
                })
            }
        };
        Ok(HirExpression {
            ty: final_type,
            kind: match kind {
                SequenceKind::Array => HirExpressionKind::Array(exprs),
                SequenceKind::Vector => HirExpressionKind::Vector(exprs),
            },
        })
    }
}
