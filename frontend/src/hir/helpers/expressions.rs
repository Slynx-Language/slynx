use common::{Operator, Span};

use crate::hir::{
    ExpressionId, SlynxHir, TypeId, VariableId,
    model::{HirExpression, HirExpressionKind, HirStatement},
};

impl SlynxHir {
    /// Creates a tuple expression with the given type and values.
    pub fn create_tuple_expression(
        &self,
        tuple_ty: TypeId,
        values: Vec<HirExpression>,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty: tuple_ty,
            kind: HirExpressionKind::Tuple(values),
            span,
        }
    }
    /// Creates a field access expression on `parent` at the given `field` index.
    pub fn create_field_access_expression(
        &self,
        parent: HirExpression,
        field: usize,
        ty: TypeId,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty,
            kind: HirExpressionKind::FieldAccess {
                expr: Box::new(parent),
                field_index: field,
            },
            span,
        }
    }
    /// Creates a boolean literal expression.
    pub fn create_boolean_expression(&self, b: bool, span: Span) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty: self.bool_type(),
            kind: HirExpressionKind::Bool(b),
            span,
        }
    }
    /// Creates a string literal expression.
    pub fn create_strliteral_expression(&self, s: String, span: Span) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty: self.str_type(),
            kind: HirExpressionKind::StringLiteral(s),
            span,
        }
    }
    /// Creates an identifier expression referencing the given variable.
    pub fn create_identifier_expression(
        &self,
        var: VariableId,
        ty: TypeId,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty,
            kind: HirExpressionKind::Identifier(var),
            span,
        }
    }
    /// Creates an int expression that must be inferred.
    pub fn create_int_expression(&self, i: i32, span: Span) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Int(i),
            id: ExpressionId::new(),
            ty: self.infer_type(),
            span,
        }
    }

    /// Creates a float expression.
    pub fn create_float_expression(&self, float: f32, span: Span) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Float(float),
            id: ExpressionId::new(),
            ty: self.infer_type(),
            span,
        }
    }
    /// Creates a binary expression.
    pub fn create_binary_expression(
        &self,
        left: HirExpression,
        right: HirExpression,
        operator: Operator,
        ty: TypeId,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Binary {
                lhs: Box::new(left),
                op: operator,
                rhs: Box::new(right),
            },
            id: ExpressionId::new(),
            ty,
            span,
        }
    }
    /// Creates a if expression.
    pub fn create_if_expression(
        &self,
        condition: HirExpression,
        then_body: Vec<HirStatement>,
        else_body: Option<Vec<HirStatement>>,
        ty: TypeId,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::If {
                condition: Box::new(condition),
                then_branch: then_body,
                else_branch: else_body,
            },
            id: ExpressionId::new(),
            ty,
            span,
        }
    }
}
