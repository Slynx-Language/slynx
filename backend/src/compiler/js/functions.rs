use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_ast::{
    AssignPat, BindingIdent, BlockStmt, Decl, EmptyStmt, FnDecl, Function, Param, Pat, Stmt,
};

use crate::{
    compiler::{js::WebCompiler, slynx_compiler::SlynxCompiler},
    
};
use middleend::intermediate::{
        IntermediateRepr,
        context::{IntermediateContext, IntermediateContextType, IntermediateProperty},
        id::ContextHandle,
        node::IntermediateInstruction,
    };
impl WebCompiler {
    pub fn retrieve_component_params(
        &mut self,
        properties: &[IntermediateProperty],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Vec<Param> {
        properties
            .iter()
            .enumerate()
            .map(|(idx, prop)| {
                let ident = Pat::Ident(BindingIdent {
                    id: self.contexts[handle.0]
                        .create_property(prop.id, &format!("a{idx}"))
                        .clone(),
                    type_ann: None,
                });
                Param {
                    pat: if let Some(default) = prop.default_value {
                        Pat::Assign(AssignPat {
                            span: DUMMY_SP,
                            left: Box::new(ident),
                            right: Box::new(self.compile_expression(
                                &ctx.exprs[default],
                                ctx,
                                ir,
                                handle,
                            )),
                        })
                    } else {
                        ident
                    },
                    span: DUMMY_SP,
                    decorators: Vec::new(),
                }
            })
            .collect()
    }

    pub fn compile_component_body(&mut self) -> BlockStmt {
        BlockStmt {
            stmts: vec![Stmt::Empty(EmptyStmt { span: DUMMY_SP })],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
        }
    }

    pub fn compile_component(
        &mut self,
        properties: &[IntermediateProperty],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) {
        let params = self.retrieve_component_params(properties, ctx, ir, handle);
        let func = Function {
            params,
            is_async: false,
            is_generator: false,
            decorators: Vec::new(),
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            type_params: None,
            return_type: None,
            body: Some(self.compile_component_body()),
        };
        let f = Decl::Fn(FnDecl {
            ident: self.context_names[handle.0].clone(),
            declare: false,
            function: Box::new(func),
        });
        self.script.body.push(Stmt::Decl(f));
    }

    pub fn compile_func_body(
        &mut self,
        instructions: &[IntermediateInstruction],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> BlockStmt {
        let stmts = self.compile_instructions(instructions, ctx, ir, handle);

        BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            stmts,
        }
    }

    pub fn compile_function(
        &mut self,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) {
        let IntermediateContextType::Function {
            args, instructions, ..
        } = &ctx.ty
        else {
            unreachable!();
        };

        let func = Function {
            is_generator: false,
            is_async: false,
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            type_params: None,
            return_type: None,
            decorators: Vec::new(),
            params: args
                .iter()
                .enumerate()
                .map(|(idx, arg)| Param {
                    pat: Pat::Ident(BindingIdent {
                        id: self.contexts[handle.0]
                            .create_variable(arg.1, &format!("p{idx}"))
                            .clone(),
                        type_ann: None,
                    }),
                    span: DUMMY_SP,
                    decorators: Vec::new(),
                })
                .collect(),
            body: Some(self.compile_func_body(instructions, ctx, ir, handle)),
        };

        self.script.body.push(Stmt::Decl(Decl::Fn(FnDecl {
            ident: self.context_names[handle.0].clone(),
            declare: false,
            function: Box::new(func),
        })));
    }
}
