use common::Span;

use crate::{
    HIRError, Result,
    builders::HirQueueBuilder,
    term::{Term, TermId, TermNode},
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
            (a, b)
                if let Some(a) = a.is_application()
                    && let Some(b) = b.is_application() =>
            {
                let target = self.unify_terms(queue, a.data.0, b.data.0, span)?;
                let args = a
                    .data
                    .1
                    .iter()
                    .zip(b.data.1)
                    .map(|(a, b)| self.unify_terms(queue, *a, *b, span))
                    .collect::<Result<Vec<TermId>>>()?;
                Ok(queue.hir.types.create_type(Term::application(target, args)))
            }
            (a, b)
                if let Some(aext) = a.is_extension()
                    && let Some(bext) = b.is_extension()
                    && aext.type_id() == bext.type_id() =>
            {
                if aext.dyn_eq(&*bext) {
                    return Ok(a.data);
                }
                let bchildren = bext.children();
                if aext.children().len() != bchildren.len() {
                    return Err(HIRError::unexpected_type(a.data, b.data, span));
                }

                let mut counter = 0;
                let aext_mapped = aext.try_map_children(&mut |child| {
                    counter += 1;
                    self.unify_terms(queue, child, bchildren[counter], span)
                })?;
                let term = Term::extension(aext_mapped);
                let target = queue.hir.types.create_type(term);
                Ok(target)
            }
            (received, expected) => Err(HIRError::unexpected_type(
                received.data,
                expected.data,
                span,
            )),
        }
    }
}
