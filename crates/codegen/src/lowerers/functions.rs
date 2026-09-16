use std::ops::{Deref, DerefMut};

use common::{Spanned, pool::PoolId};
use slynx_hir::{HirStatement, VariableId};
use slynx_ir::{Function, FunctionBuilder, IRPointer, IRTypeId, SlynxIR, Value};

use crate::{CodegenError, TypeId, lowerers::LoweringState};

/// Per-function state during HIR-to-IR lowering.
pub struct FunctionContext<'a> {
    function_builder: FunctionBuilder<'a>,
    args: Vec<(VariableId, Value)>,
}

impl<'a> FunctionContext<'a> {
    pub(crate) fn new(function_builder: FunctionBuilder<'a>) -> Self {
        Self {
            function_builder,
            args: Vec::new(),
        }
    }

    pub fn get_variable(&self, id: VariableId) -> Option<Value> {
        self.args.iter().find_map(|v| (v.0 == id).then_some(v.1))
    }

    pub fn add_variable(&mut self, id: VariableId, value: Value) {
        self.args.push((id, value));
    }

    pub fn ir(&mut self) -> &mut SlynxIR {
        self.function_builder.ir()
    }

    /// Finalize the function and return its pointer.
    /// Consumes this context.
    pub fn finish(self) -> IRPointer<Function, 1> {
        self.function_builder.generate()
    }
}

impl<'a> Deref for FunctionContext<'a> {
    type Target = FunctionBuilder<'a>;
    fn deref(&self) -> &Self::Target {
        &self.function_builder
    }
}

impl<'a> DerefMut for FunctionContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.function_builder
    }
}

impl<'a> LoweringState<'a> {
    fn map_function_type(
        &mut self,
        func_ty: TypeId,
        ir: &mut SlynxIR,
    ) -> Result<(Vec<IRTypeId>, IRTypeId), CodegenError> {
        let Some(viewer) = self.hir.view(func_ty).is_function() else {
            unreachable!("Initialize function should initialize with the type of a function");
        };
        let (args, return_type) = (viewer.arguments().to_vec(), viewer.return_type());
        let args = args
            .iter()
            .map(|v| self.types.get_or_create_ir_type(*v, ir))
            .collect::<Result<Vec<_>, CodegenError>>()?;
        let return_type = self.types.get_or_create_ir_type(return_type, ir)?;
        Ok((args, return_type))
    }

    pub(crate) fn map_function_arguments<'b>(
        &mut self,
        context: &mut FunctionContext<'b>,
        args: &[VariableId],
    ) {
        let arg_values = context.arguments().to_vec();
        for (variable, value) in args.iter().zip(arg_values) {
            context.add_variable(*variable, value);
        }
    }

    pub(crate) fn initialize_function(
        &mut self,
        fptr: IRPointer<Function, 1>,
        func_ty: TypeId,
        statements: &[Spanned<PoolId<HirStatement>>],
        args: &[VariableId],
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let (arg_types, return_type) = self.map_function_type(func_ty, ir)?;
        let builder = ir.build_function(fptr);
        let mut context = FunctionContext::new(builder);

        // Switch to entry block
        let entry = context.create_label("entry");
        context.switch_to_block(entry).unwrap();

        context.set_function_type(arg_types, return_type);
        // Emit function arg instructions and set the function type

        self.map_function_arguments(&mut context, args);

        self.lower_body(&mut context, statements)?;
        context.finish();
        Ok(())
    }

    fn lower_body<'b>(
        &mut self,
        ctx: &mut FunctionContext<'b>,
        statements: &[Spanned<PoolId<HirStatement>>],
    ) -> Result<(), CodegenError> {
        for (idx, statement) in statements.iter().enumerate() {
            if let Some(value) = self.lower_statement(*statement, ctx)?
                && idx == statements.len() - 1
            {
                ctx.ret(value);
            }
        }
        Ok(())
    }
}