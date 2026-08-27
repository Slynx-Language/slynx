use std::collections::HashMap;

use common::{Span, Spanned, VisibilityModifier, pool::DedupPoolId};
use slynx_parser::{NamedExpr, Type, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, builders::HirQueueBuilder,
};

use super::ExpressionBuilder;

impl ExpressionBuilder {

    pub(super) fn build_object(
        &mut self,
        queue: &HirQueueBuilder,
        name: Spanned<DedupPoolId<Type>>,
        fields: &[Spanned<NamedExpr>],
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
        context: &TypeContext,
    ) -> Result<HirExpression> {

        let ty = if let Some(self_type) = &self.self_type {
            queue.find_self_type(name.data, *self_type)
        }else {
            queue.get_node(self.file()).find_type(name, context)?.1
        };
        let ty_view = queue.hir.view(ty);
        let deref = ty_view.dereference();
        let obj = deref
            .is_struct()
            .expect("Expected name to generate a struct type");

        // A generic object literal written without explicit type
        // arguments (`Wrapper(data: 5)`) resolves to the raw template
        // struct type, leaving its fields as `GenericParam`s. When the
        // expected type is a concrete reference to the same struct,
        // adopt it so the template's generic fields resolve against the
        // type arguments.
        let ty = match ty_view.raw() {
            HirType::Reference { .. } => ty,
            _ => match expected {
                Some(expected_ty)
                    if queue.hir.view(expected_ty).dereference().data == deref.data
                        && matches!(
                            queue.hir.view(expected_ty).raw(),
                            HirType::Reference { .. }
                        ) =>
                {
                    expected_ty
                }
                _ => ty,
            },
        };
        let ty_view = queue.hir.view(ty);
        let generics: &[DedupPoolId<HirType>] = match ty_view.raw() {
            HirType::Reference { generics, .. } => generics.as_slice(),
            _ => &[],
        };

        let type_names: HashMap<_, _> = obj
            .fields()
            .iter()
            .enumerate()
            .map(|(i, s)| (s.data, (i, s.visibility)))
            .collect();
        let mut ordered = vec![None; obj.fields().len()];

        for field in fields {
            let (idx, visibility) =
                *type_names
                    .get(&field.data.name)
                    .ok_or(HIRError::property_unrecognized(
                        ty,
                        vec![field.data.name],
                        field.span,
                    ))?;
            if visibility != VisibilityModifier::Public {
                return Err(HIRError::not_visible_property(field.data.name, field.span));
            }

            if ordered[idx].replace(field).is_some() {
                return Err(HIRError::already_defined(field.data.name, field.span));
            }
        }

        let fields = {
            let mut fields = Vec::with_capacity(ordered.len());

            let mut missing = Vec::new();
            for (i, field) in ordered.into_iter().enumerate() {
                match field {
                    Some(field) => fields.push({
                        let fieldname = field.data.name;
                        let (idx, _) = type_names
                            .get(&fieldname)
                            .expect("Field name should've been added into type names");

                        let field_ty = queue.substitute_generics(generics, obj.field_types()[*idx]);
                        self.build_expression(queue, field.data.expr, Some(field_ty), context)?
                    }),
                    None => missing.push(obj.fields()[i].data),
                }
            }

            if !missing.is_empty() {
                return Err(HIRError::missing_properties(missing, span));
            }
            fields
        };

        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::Object { name: ty, fields },
        })
    }
}
