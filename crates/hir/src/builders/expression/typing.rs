use common::{Span, pool::DedupPoolId};

use crate::{HIRError, HirType, Result, builders::HirQueueBuilder};

use super::ExpressionBuilder;

impl ExpressionBuilder {
    pub(super) fn unify_types(
        &self,
        queue: &HirQueueBuilder,
        received: DedupPoolId<HirType>,
        expected: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<DedupPoolId<HirType>> {
        if received == expected {
            return Ok(received);
        }
        match (
            queue.hir.view(received).dereference(),
            queue.hir.view(expected).dereference(),
        ) {
            (a, b) if *a == *b => Ok(a.data),
            (a, b)
                if let Some(inner) = a.is_nullable()
                    && inner == b.data =>
            {
                Ok(a.data)
            }
            (b, a)
                if let Some(inner) = a.is_nullable()
                    && inner == b.data =>
            {
                Ok(a.data)
            }
            (a, b) if let HirType::GenericParam { .. } = a.raw() => Ok(b.data),
            (b, a) if let HirType::GenericParam { .. } = a.raw() => Ok(b.data),
            (received, expected) => Err(HIRError::unexpected_type(
                received.data,
                expected.data,
                span,
            )),
        }
    }
}
