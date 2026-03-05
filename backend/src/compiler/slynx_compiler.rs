use middleend::intermediate::{
    IntermediateRepr, context::IntermediateContext, expr::IntermediateExpr, id::ContextHandle,
    node::IntermediateInstruction,
};

///A trait to define how the Slynx IR will be compiled
pub trait SlynxCompiler {
    type ExpressionType;
    type StatementType;
    ///Compiles the provided `instructions`. The `ctx` and `ir` are the context of the IR and the IR itself. The provided `handle`
    ///is the handle of the current compiling context
    fn compile_instructions(
        &mut self,
        instructions: &[IntermediateInstruction],
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Vec<Self::StatementType>;

    ///Compiles the provided `expr` using the given `ctx` and `ir`. The provided `handle` is the handle
    ///of the context of compiler itself, instead of the IR
    fn compile_expression(
        &mut self,
        expr: &IntermediateExpr,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        handle: ContextHandle,
    ) -> Self::ExpressionType;
    fn compile(self, ir: IntermediateRepr) -> Vec<u8>;
}
