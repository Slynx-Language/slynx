use common::{Span, pool::DedupPoolId};

use crate::{HIRError, HirType, Result, builders::HirQueueBuilder};

use super::ExpressionBuilder;

impl HirQueueBuilder<'_> {
    ///Replaces every [`HirType::GenericParam`] inside `field_ty` with the matching
    ///type argument from a generic reference's `generics` list, recursing through
    ///container types. Used to type field accesses on generic struct references
    ///concretely: `a.data` on a `Wrapper<Wrapper<int>>` is `Wrapper<int>`, not the
    ///template's `GenericParam(0)`.
    pub(super) fn substitute_generics(
        &self,
        generics: &[DedupPoolId<HirType>],
        field_ty: DedupPoolId<HirType>,
    ) -> DedupPoolId<HirType> {
        match self.hir.view(field_ty).raw() {
            HirType::GenericParam { index, .. } => {
                generics.get(*index as usize).copied().unwrap_or(field_ty)
            }
            HirType::Array(inner, len) => {
                let inner = self.substitute_generics(generics, *inner);
                self.hir.create_type(HirType::Array(inner, *len))
            }
            HirType::Vector(inner) => {
                let inner = self.substitute_generics(generics, *inner);
                self.hir.create_type(HirType::Vector(inner))
            }
            HirType::Nullable(inner) => {
                let inner = self.substitute_generics(generics, *inner);
                self.hir.create_type(HirType::Nullable(inner))
            }
            HirType::Tuple(tuple) => {
                let fields = self
                    .hir
                    .view(*tuple)
                    .fields()
                    .iter()
                    .map(|field| self.substitute_generics(generics, *field))
                    .collect::<Vec<_>>();
                self.hir.create_tuple_type(fields)
            }
            HirType::Function(function) => {
                let function_view = self.hir.view(*function);
                let args = function_view
                    .arguments()
                    .iter()
                    .map(|arg| self.substitute_generics(generics, *arg))
                    .collect::<Vec<_>>();
                let ret = self.substitute_generics(generics, function_view.return_type());
                self.hir.create_function_type(args, ret)
            }
            HirType::Reference {
                rf,
                generics: inner_generics,
            } => {
                let rf = self.substitute_generics(generics, *rf);
                let mut new_generics = *inner_generics;
                for slot in &mut new_generics {
                    if !slot.is_null() {
                        *slot = self.substitute_generics(generics, *slot);
                    }
                }
                self.hir.create_type(HirType::Reference {
                    rf,
                    generics: new_generics,
                })
            }
            _ => field_ty,
        }
    }
}

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
