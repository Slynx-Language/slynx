use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrayLit, Expr, ExprOrSpread, IdentName, Lit, MemberExpr, MemberProp, ObjectLit, PropOrSpread,
};

use crate::{
    compiler::{js::WebCompiler, slynx_compiler::SlynxCompiler},
    intermediate::{
        IntermediateRepr,
        context::IntermediateContext,
        expr::{NativeComponent, NativeComponentKind},
        id::{ContextHandle, ValueId},
    },
};

impl WebCompiler {
    fn compile_array_of_optional_exprs(
        &mut self,
        values: &[Option<ValueId>],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Expr {
        Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: values
                .iter()
                .map(|value| {
                    Some(ExprOrSpread {
                        spread: None,
                        expr: if let Some(value) = value {
                            Box::new(self.compile_expression(&ctx.exprs[*value], ctx, ir, handle))
                        } else {
                            Self::undefined()
                        },
                    })
                })
                .collect(),
        })
    }

    fn compile_array_of_exprs(
        &mut self,
        values: &[ValueId],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Expr {
        Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: values
                .iter()
                .map(|value| {
                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(self.compile_expression(
                            &ctx.exprs[*value],
                            ctx,
                            ir,
                            handle,
                        )),
                    })
                })
                .collect(),
        })
    }

    pub fn compile_native(
        &mut self,
        native: &NativeComponent,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Expr {
        let mut props = Vec::new();
        match &native.kind {
            NativeComponentKind::Text { text } => {
                props.push(PropOrSpread::Prop(Box::new(Self::prop(
                    "kind",
                    Expr::Lit(Lit::Str("Text".into())),
                ))));
                props.push(PropOrSpread::Prop(Box::new(Self::prop(
                    "text",
                    self.compile_expression(&ctx.exprs[*text], ctx, ir, handle),
                ))));
            }
            NativeComponentKind::Rect { children } => {
                props.push(PropOrSpread::Prop(Box::new(Self::prop(
                    "kind",
                    Expr::Lit(Lit::Str("Rect".into())),
                ))));
                props.push(PropOrSpread::Prop(Box::new(Self::prop(
                    "children",
                    self.compile_array_of_exprs(children, ctx, ir, handle),
                ))));
            }
        }
        props.push(PropOrSpread::Prop(Box::new(Self::prop(
            "props",
            self.compile_array_of_optional_exprs(&native.props, ctx, ir, handle),
        ))));
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
        })
    }

    pub fn compile_struct(
        &mut self,
        exprs: &[ValueId],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Expr {
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: exprs
                .iter()
                .enumerate()
                .map(|(idx, ptr)| {
                    PropOrSpread::Prop(Box::new(Self::prop(
                        &format!("f{idx}"),
                        self.compile_expression(&ctx.exprs[*ptr], ctx, ir, handle),
                    )))
                })
                .collect(),
        })
    }
    ///Compiles a field expression such as `variable.b`
    pub fn compile_field_access(
        &mut self,
        parent: ValueId,
        field: usize,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Expr {
        let parent = self.compile_expression(&ctx.exprs[parent], ctx, ir, handle);
        Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(parent),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: Atom::new(format!("f{field}")),
            }),
        })
    }
}
