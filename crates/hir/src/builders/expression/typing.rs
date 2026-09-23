use common::Span;

use crate::{
    HIRError, Result,
    builders::HirQueueBuilder,
    term::{TermId, TermNode},
};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(super) fn unify_terms(
        &self,
        queue: &HirQueueBuilder,
        received: TermId,
        expected: TermId,
        span: Span,
    ) -> Result<TermId> {
        if received == expected {
            return Ok(received);
        }
        match (
            queue.hir.view(received).dereference(),
            queue.hir.view(expected).dereference(),
        ) {
            (a, b) if a.data == b.data => Ok(a.data),
            (a, b) if let TermNode::Var { .. } = a.raw().node() => Ok(b.data),
            (b, a) if let TermNode::Var { .. } = a.raw().node() => Ok(b.data),
            (received, expected) => Err(HIRError::unexpected_type(
                received.data,
                expected.data,
                span,
            )),
        }
    }
}
