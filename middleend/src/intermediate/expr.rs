use crate::{
    intermediate::{
        id::{ContextHandle, TyId, ValueId, VarId},
        string::StringHandle,
    },
    
};
use common::ast::Operator;

///A Native component that is not user-defined
#[derive(Debug, Clone)]
pub enum NativeComponentKind {
    Text { text: ValueId },
    Rect { children: Vec<ValueId> },
}

#[derive(Debug, Clone)]

pub struct NativeComponent {
    pub kind: NativeComponentKind,
    pub props: Vec<Option<ValueId>>,
}

#[derive(Debug, Clone)]
pub struct IntermediateExpr {
    pub kind: IntermediateExprKind,
}

#[derive(Debug, Clone)]
pub enum IntermediateExprKind {
    Bool(bool),
    Int(i32),
    Float(f32),
    StringLiteral(StringHandle),
    Struct {
        id: TyId,
        exprs: Vec<ValueId>,
    },
    FieldAccess {
        parent: ValueId,
        ///The field index being accessed
        field: usize,
    },
    Binary {
        ///Inside the IntermediateRepr, the index of the lhs expression
        lhs: ValueId,
        ///Inside the IntermediateRepr, the index of the rhs expression
        rhs: ValueId,
        operator: Operator,
    },
    Identifier(VarId),
    ///An component expresssion. The props are the public children that may require some input. A None value will result in passing to them undefined
    ///and a Some(value) will pass to them the expression on the `value` of the current context
    ///The children are the children for this component, so, an array of indices for more component expressions inside the ccurrent context
    Component {
        id: ContextHandle,
        props: Vec<Option<ValueId>>,
        children: Vec<ValueId>,
    },
    FunctionCall {
        id: ContextHandle,
        args: Vec<ValueId>,
    },
    Native(NativeComponent),
}

impl IntermediateExpr {
    ///Creates a native `text` component with the provided `text`
    pub fn native_text(text: ValueId, props: Vec<Option<ValueId>>) -> Self {
        Self {
            kind: IntermediateExprKind::Native(NativeComponent {
                kind: NativeComponentKind::Text { text },
                props,
            }),
        }
    }
    ///Creates a native `rect` component with the provided `children`
    pub fn native_rect(children: Vec<ValueId>, props: Vec<Option<ValueId>>) -> Self {
        Self {
            kind: IntermediateExprKind::Native(NativeComponent {
                kind: NativeComponentKind::Rect { children },
                props,
            }),
        }
    }
    pub fn field(expr: ValueId, field: usize) -> Self {
        Self {
            kind: IntermediateExprKind::FieldAccess {
                parent: expr,
                field,
            },
        }
    }
    pub fn strct(ty: TyId, fields: Vec<ValueId>) -> Self {
        Self {
            kind: IntermediateExprKind::Struct {
                id: ty,
                exprs: fields,
            },
        }
    }
    pub fn component(
        id: ContextHandle,
        props: Vec<Option<ValueId>>,
        children: Vec<ValueId>,
    ) -> Self {
        Self {
            kind: IntermediateExprKind::Component {
                id,
                props,
                children,
            },
        }
    }

    pub fn functioncall(id: ContextHandle, args: Vec<ValueId>) -> Self {
        Self {
            kind: IntermediateExprKind::FunctionCall { id, args },
        }
    }

    pub fn identifier(varid: VarId) -> Self {
        Self {
            kind: IntermediateExprKind::Identifier(varid),
        }
    }
    pub fn int(int: i32) -> Self {
        Self {
            kind: IntermediateExprKind::Int(int),
        }
    }
    pub fn float(float: f32) -> Self {
        Self {
            kind: IntermediateExprKind::Float(float),
        }
    }
    pub fn str(handle: StringHandle) -> Self {
        Self {
            kind: IntermediateExprKind::StringLiteral(handle),
        }
    }
    pub fn bin(lhs: ValueId, rhs: ValueId, operator: Operator) -> Self {
        Self {
            kind: IntermediateExprKind::Binary { lhs, rhs, operator },
        }
    }
    pub fn bool(b: bool) -> Self {
        Self {
            kind: IntermediateExprKind::Bool(b),
        }
    }
}
