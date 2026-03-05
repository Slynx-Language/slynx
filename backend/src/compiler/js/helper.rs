use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{Expr, KeyValueProp, Prop};

use crate::compiler::js::WebCompiler;

impl WebCompiler {
    ///Creates an 'undefined' expression
    pub fn undefined() -> Box<Expr> {
        Expr::undefined(DUMMY_SP)
    }

    ///Creates a new property with provided `name` and `value`
    pub fn prop(name: &str, value: Expr) -> Prop {
        Prop::KeyValue(KeyValueProp {
            key: swc_ecma_ast::PropName::Ident(swc_ecma_ast::IdentName {
                span: DUMMY_SP,
                sym: Atom::new(name),
            }),
            value: value.into(),
        })
    }
}
