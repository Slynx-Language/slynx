mod contexts;
mod expr;
mod functions;
mod helper;
mod types;

use std::rc::Rc;

use swc_atoms::Atom;
use swc_common::{DUMMY_SP, SourceMap, SyntaxContext};
use swc_ecma_ast::{
    AssignExpr, AssignOp, AssignTarget, BinExpr, BinaryOp, BindingIdent, CallExpr, Callee, Decl,
    Expr, ExprOrSpread, ExprStmt, Ident, Lit, MemberExpr, MemberProp, Number, Pat, Program,
    ReturnStmt, Script, SimpleAssignTarget, Stmt, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_ecma_codegen::{Config, Emitter, text_writer::JsWriter};

use crate::{
    compiler::{js::contexts::JsFunction, slynx_compiler::SlynxCompiler},
    
};

use middleend::intermediate::{
        IntermediateRepr,
        context::IntermediateContext,
        expr::{IntermediateExpr, IntermediateExprKind},
        id::ContextHandle,
        node::{IntermediateInstruction, IntermediateInstructionKind, IntermediatePlace},
    };
use common::ast::Operator;
#[derive(Debug, Default)]
pub struct WebCompiler {
    script: Script,
    contexts: Vec<JsFunction>,
    context_names: Vec<Ident>,
}

pub fn create_ident(s: &str) -> Ident {
    Ident::new_no_ctxt(Atom::new(s), DUMMY_SP)
}

impl WebCompiler {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            context_names: Vec::new(),
            script: Script::default(),
        }
    }
    ///Creates the new component name, binds it to be the name of the provided `id` and returns it
    pub fn retrieve_next_component_name(&mut self, handle: ContextHandle) {
        assert!(handle.0 == self.context_names.len());
        let name = format!("c{}", handle.0);
        self.context_names.push(create_ident(&name));
    }

    ///Creates the new func name, binds it to be the name of the provided `id` and returns it
    pub fn retrieve_next_func_name(&mut self, handle: ContextHandle) {
        assert!(handle.0 == self.context_names.len());
        let name = format!("c{}", handle.0);
        self.context_names.push(create_ident(&name));
    }
}

impl SlynxCompiler for WebCompiler {
    type ExpressionType = Expr;
    type StatementType = Stmt;
    fn compile_instructions(
        &mut self,
        instructions: &[IntermediateInstruction],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Vec<Self::StatementType> {
        let mut out = Vec::with_capacity(instructions.len());
        for inst in instructions {
            let stmt = match &inst.kind {
                IntermediateInstructionKind::Ret(id) => {
                    let expr = self.compile_expression(&ctx.exprs[*id], ctx, ir, handle);
                    Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(Box::new(expr)),
                    })
                }
                IntermediateInstructionKind::Alloc(v) => {
                    let varslen = self.contexts[handle.0].variables.len();
                    let ident = self.contexts[handle.0].create_variable(*v, &format!("v{varslen}"));
                    Stmt::Decl(Decl::Var(Box::new(VarDecl {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::default(),
                        decls: vec![VarDeclarator {
                            span: DUMMY_SP,
                            name: Pat::Ident(BindingIdent {
                                type_ann: None,
                                id: ident.clone(),
                            }),
                            init: None,
                            definite: true,
                        }],
                        kind: VarDeclKind::Let,
                        declare: false,
                    })))
                }
                IntermediateInstructionKind::Move { target, value } => match target {
                    IntermediatePlace::Local(n) => Stmt::Expr(ExprStmt {
                        expr: Box::new(Expr::Assign(AssignExpr {
                            span: DUMMY_SP,
                            op: AssignOp::Assign,
                            left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent {
                                id: {
                                    let ctx = &mut self.contexts[handle.0];
                                    ctx.retrieve_varname(*n).unwrap().clone()
                                },
                                type_ann: None,
                            })),
                            right: Box::new(self.compile_expression(
                                &ctx.exprs[*value],
                                ctx,
                                ir,
                                handle,
                            )),
                        })),
                        span: DUMMY_SP,
                    }),
                    IntermediatePlace::Field { field, parent } => Stmt::Expr(ExprStmt {
                        expr: Box::new(Expr::Assign(AssignExpr {
                            span: DUMMY_SP,
                            op: AssignOp::Assign,
                            left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                                span: DUMMY_SP,
                                obj: Box::new(Expr::Ident({
                                    self.contexts[handle.0]
                                        .retrieve_varname(*parent)
                                        .cloned()
                                        .unwrap()
                                })),
                                prop: MemberProp::Ident(format!("f{field}").into()),
                            })),
                            right: Box::new(self.compile_expression(
                                &ctx.exprs[*value],
                                ctx,
                                ir,
                                handle,
                            )),
                        })),
                        span: DUMMY_SP,
                    }),
                },
                IntermediateInstructionKind::Read(v) => Stmt::Expr(ExprStmt {
                    expr: Box::new(Expr::Ident(
                        self.contexts[handle.0]
                            .retrieve_varname(*v)
                            .cloned()
                            .unwrap(),
                    )),
                    span: DUMMY_SP,
                }),
            };
            out.push(stmt);
        }
        out
    }
    fn compile_expression(
        &mut self,
        expr: &IntermediateExpr,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Self::ExpressionType {
        match &expr.kind {
            IntermediateExprKind::Identifier(name) => Expr::Ident(
                self.contexts[handle.0]
                    .retrieve_varname(*name)
                    .unwrap()
                    .clone(),
            ),
            IntermediateExprKind::Int(int) => Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: *int as f64,
                raw: None,
            })),
            IntermediateExprKind::StringLiteral(s) => Expr::Lit(Lit::Str(ir.strings[s].into())),
            IntermediateExprKind::FunctionCall { id, args } => {
                let callee = Callee::Expr(Box::new(Expr::Ident(self.context_names[id.0].clone())));
                let args = args
                    .iter()
                    .map(|expr| ExprOrSpread {
                        spread: None,
                        expr: Box::new(self.compile_expression(&ctx.exprs[*expr], ctx, ir, handle)),
                    })
                    .collect();
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee,
                    args,
                    type_args: None,
                })
            }
            IntermediateExprKind::Component {
                props: args, id, ..
            } => {
                let callee = Callee::Expr(Box::new(Expr::Ident(self.context_names[id.0].clone())));
                let args = {
                    if args.iter().all(|v| v.is_none()) {
                        Vec::new()
                    } else {
                        args.iter()
                            .map(|expr| ExprOrSpread {
                                spread: None,
                                expr: if let Some(expr_idx) = expr {
                                    Box::new(self.compile_expression(
                                        &ctx.exprs[*expr_idx],
                                        ctx,
                                        ir,
                                        handle,
                                    ))
                                } else {
                                    Self::undefined()
                                },
                            })
                            .collect()
                    }
                };
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee,
                    args,
                    type_args: None,
                })
            }
            IntermediateExprKind::Struct { exprs, .. } => {
                self.compile_struct(exprs, ctx, ir, handle)
            }
            IntermediateExprKind::FieldAccess { parent, field } => {
                self.compile_field_access(*parent, *field, ctx, ir, handle)
            }
            IntermediateExprKind::Binary { lhs, rhs, operator } => Expr::Bin(BinExpr {
                span: DUMMY_SP,
                op: operator_to_binary_op(operator),
                left: Box::new(self.compile_expression(&ctx.exprs[*lhs], ctx, ir, handle)),
                right: Box::new(self.compile_expression(&ctx.exprs[*rhs], ctx, ir, handle)),
            }),
            IntermediateExprKind::Float(n) => Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: *n as f64,
                raw: None,
            })),
            IntermediateExprKind::Bool(b) => Expr::Lit(Lit::Bool((*b).into())),
            IntermediateExprKind::Native(native) => self.compile_native(native, ctx, ir, handle),
        }
    }

    ///The flattener has everything it's required
    fn compile(mut self, ir: IntermediateRepr) -> Vec<u8> {
        for ctx in ir.contexts.iter() {
            let mut context = JsFunction::new(ctx.id);
            self.hoist_ctx(ctx, &mut context);
            self.contexts.push(context);
        }
        for ctx in ir.contexts.iter() {
            self.compile_ctx(ctx, &ir, ctx.id);
        }
        {
            let mut buf = Vec::new();
            let cm = Rc::new(SourceMap::default());
            let mut writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
            writer.set_indent_str("  ");
            let mut emitter = Emitter {
                cfg: Config::default(),
                cm: cm.clone(),
                comments: None,
                wr: writer,
            };
            emitter.emit_program(&Program::Script(self.script)).unwrap();
            String::from_utf8(buf).unwrap().into_bytes()
        }
    }
}

fn operator_to_binary_op(operator: &Operator) -> BinaryOp {
    match operator {
        Operator::LogicAnd => BinaryOp::LogicalAnd,
        Operator::LogicOr => BinaryOp::LogicalOr,
        Operator::Add => BinaryOp::Add,
        Operator::Sub => BinaryOp::Sub,
        Operator::Star => BinaryOp::Mul,
        Operator::Slash => BinaryOp::Div,
        Operator::Equals => BinaryOp::EqEqEq,
        Operator::GreaterThan => BinaryOp::Gt,
        Operator::LessThan => BinaryOp::Lt,
        Operator::GreaterThanOrEqual => BinaryOp::GtEq,
        Operator::LessThanOrEqual => BinaryOp::LtEq,
    }
}

#[cfg(test)]
mod tests {
    use super::WebCompiler;
    use crate::{
        compiler::{js::contexts::JsFunction, slynx_compiler::SlynxCompiler},
        
    };
    use middleend::{
        intermediate::{
            IntermediateRepr,
            context::IntermediateContext,
            expr::IntermediateExpr,
            id::{ContextHandle, VarId},
            node::IntermediateInstruction,
            types::IntermediateType,
        },
    };
    use swc_ecma_ast::{Expr, Stmt};

    #[test]
    fn compile_expression_handles_native_component() {
        let handle = ContextHandle(0);
        let mut compiler = WebCompiler::new();
        compiler.contexts.push(JsFunction::new(handle));

        let mut ctx = IntermediateContext::new_function(
            handle,
            "f".to_string(),
            Vec::new(),
            IntermediateType::Void,
        );
        let text_expr = ctx.insert_expr(IntermediateExpr::int(1));
        let native_expr = ctx.insert_expr(IntermediateExpr::native_text(text_expr, vec![None]));

        let ir = IntermediateRepr::new();
        let compiled = compiler.compile_expression(&ctx.exprs[native_expr], &ctx, &ir, handle);
        assert!(matches!(compiled, Expr::Object(_)));
    }

    #[test]
    fn compile_instructions_handles_read_instruction() {
        let handle = ContextHandle(0);
        let mut compiler = WebCompiler::new();
        let mut js_ctx = JsFunction::new(handle);
        let var = VarId::new();
        js_ctx.create_variable(var, "v0");
        compiler.contexts.push(js_ctx);

        let ctx = IntermediateContext::new_function(
            handle,
            "f".to_string(),
            Vec::new(),
            IntermediateType::Void,
        );
        let instructions = vec![IntermediateInstruction::read(var)];
        let ir = IntermediateRepr::new();

        let compiled = compiler.compile_instructions(&instructions, &ctx, &ir, handle);
        assert!(matches!(compiled.as_slice(), [Stmt::Expr(_)]));
    }
}
