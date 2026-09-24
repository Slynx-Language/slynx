use std::ops::{Deref, DerefMut};

use common::{Spanned, pool::PoolId};
use slynx_hir::{HirStatement, VariableId};
use slynx_ir::{Function, FunctionBuilder, IRPointer, IRTypeId, Label, SlynxIR, Value};

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

    /// Switches the active block, propagating errors from the IR builder
    /// instead of panicking on corrupt builder state.
    pub fn block(&mut self, label: IRPointer<Label, 1>) -> Result<(), CodegenError> {
        self.switch_to_block(label).map_err(CodegenError::from)
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
        let Some((args, return_type)) = self.hir.view(func_ty).is_function() else {
            unreachable!("Initialize function should initialize with the type of a function");
        };
        let args = args
            .iter()
            .map(|v| self.types.get_or_create_ir_type(*v, ir))
            .collect::<Result<Vec<_>, CodegenError>>()?;
        let return_type = self.types.get_or_create_ir_type(return_type, ir)?;
        Ok((args, return_type))
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
        context.block(entry)?;

        // Emit function arg instructions and set the function type. The arg
        // values returned here are the single source for the parameter slots.
        let arg_values = context.set_function_type(arg_types, return_type).to_vec();
        for (variable, value) in args.iter().zip(arg_values) {
            context.add_variable(*variable, value);
        }

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
