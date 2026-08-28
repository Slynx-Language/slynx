//! HirPlace construction from expressions.

use common::pool::PoolId;

use crate::{
    HirExpression, HirExpressionKind, SlynxHir, model::HirPlace,
    ownership::analysis::OwnershipAnalysis,
};

impl OwnershipAnalysis {
    /// Build a HirPlace from an expression that refers to a memory location.
    ///
    /// Returns `Some(place_id)` if the expression is a place (lvalue),
    /// `None` if it's a pure value (rvalue).
    pub fn build_place_from_expr(
        &mut self,
        hir: &SlynxHir,
        expr_id: PoolId<HirExpression>,
    ) -> Option<PoolId<HirPlace>> {
        // Check if we already built a place for this expression
        if let Some(&existing) = self.expression_places.get(&expr_id) {
            return Some(existing);
        }

        let expr = &hir[expr_id];
        let place = match &expr.kind {
            HirExpressionKind::Identifier(var_id) => HirPlace::Variable(*var_id),
            HirExpressionKind::FieldAccess {
                expr: parent_expr,
                field_index,
                field_name,
            } => {
                let parent_place = self.build_place_from_expr(hir, parent_expr.data)?;
                HirPlace::Field {
                    place: parent_place,
                    index: *field_index,
                    name: *field_name,
                }
            }
            HirExpressionKind::ArrayIndex(arr_expr, index_expr) => {
                let arr_place = self.build_place_from_expr(hir, arr_expr.data)?;
                HirPlace::Index {
                    place: arr_place,
                    index: index_expr.data,
                }
            }
            HirExpressionKind::Deref(inner_expr) => {
                let inner_place = self.build_place_from_expr(hir, inner_expr.data)?;
                HirPlace::Deref { place: inner_place }
            }
            // Function calls produce temporaries
            HirExpressionKind::FunctionCall { .. } => HirPlace::Temporary(expr_id),
            _ => return None,
        };

        let place_id = hir.places.insert(place);
        self.expression_places.insert(expr_id, place_id);
        Some(place_id)
    }
}
