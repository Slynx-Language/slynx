use std::ops::{Deref, DerefMut};

use slynx_hir::{HirStatement, HirType, SlynxHir, TypeId, VariableId};
use slynx_ir::{Function, FunctionBuilder, IRPointer, IRTypeId, SlynxIR, Value};

use crate::{Codegen, CodegenError};

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

impl Codegen {
    fn map_function_type(
        &self,
        func_ty: TypeId,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(Vec<IRTypeId>, IRTypeId), CodegenError> {
        let (args, return_type) = {
            let ty = hir.get_type(&func_ty);
            let HirType::Function { args, return_type } = &*ty else {
                unreachable!("Initialize function should initialize with the type of a function");
            };
            (args.clone(), *return_type)
        };
        let args = args
            .iter()
            .map(|v| self.get_or_create_ir_type(v, hir, ir))
            .collect::<Result<Vec<_>, CodegenError>>()?;
        let return_type = self.get_or_create_ir_type(&return_type, hir, ir)?;
        Ok((args, return_type))
    }

    pub(crate) fn map_function_arguments<'a>(
        &mut self,
        context: &mut FunctionContext<'a>,
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
        statements: &[HirStatement],
        args: &[VariableId],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let (arg_types, return_type) = self.map_function_type(func_ty, hir, ir)?;
        let builder = ir.build_function(fptr);
        let mut context = FunctionContext::new(builder);

        // Switch to entry block
        let entry = context.create_label("entry");
        context.switch_to_block(entry).unwrap();

        context.set_function_type(arg_types, return_type);
        // Emit function arg instructions and set the function type

        self.map_function_arguments(&mut context, args);

        self.lower_body(&mut context, hir, statements)?;
        context.finish();
        Ok(())
    }

    fn lower_body<'a>(
        &mut self,
        ctx: &mut FunctionContext<'a>,
        hir: &SlynxHir,
        statements: &[HirStatement],
    ) -> Result<(), CodegenError> {
        for (idx, statement) in statements.iter().enumerate() {
            if let Some(value) = self.lower_statement(statement, hir, ctx)?
                && idx == statements.len() - 1
            {
                ctx.ret(value);
            }
        }
        Ok(())
    }
}
