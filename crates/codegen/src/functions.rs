use slynx_hir::{HirStatement, HirType, SlynxHir, TypeId, VariableId};
use slynx_ir::{
    Function, FunctionBuilder, IRPointer, IRType, IRTypeId, LabelBuilder, SlynxIR, Value,
};

use crate::{Codegen, CodegenError};

/// Per-function state during HIR-to-IR lowering.
///
/// Mirrors the IR crate's [`FunctionBuilder`] + [`LabelBuilder`] pattern
/// by isolating label bookkeeping and argument metadata from the broader
/// `TempIRData`-based pipeline.  This struct owns the function pointer and
/// a snapshot of argument types, removing the need to re-query the IR at
/// later phases and reducing coupling to implicit [`TempIRData`] state.
///
/// [`FunctionBuilder`]: slynx_ir::builder::functions::FunctionBuilder
/// [`LabelBuilder`]:    slynx_ir::builder::functions::LabelBuilder
pub struct FunctionContext<'a> {
    /// Handle to the IR [`Function`] being populated.
    function_builder: FunctionBuilder<'a>,

    args: Vec<(VariableId, Value)>,
}

impl<'a> FunctionContext<'a> {
    fn new(mut function_builder: FunctionBuilder<'a>) -> Self {
        function_builder.create_label("entry");
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
    pub fn ir(&'a mut self) -> &'a mut SlynxIR {
        self.function_builder.ir()
    }
}

impl Codegen {
    ///Sets up the function type. This should be made after hoisting every type. Returns the argument types of the function, and its return type
    fn mutate_function_type(
        &self,
        func_ty: TypeId,
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(Vec<IRTypeId>, IRTypeId), CodegenError> {
        let (args, return_type) = {
            let ty = hir.get_type(&func_ty);

            let HirType::Function { args, return_type } = ty else {
                unreachable!("Initialize function should initialize with the type of a function");
            };
            let args = args
                .iter()
                .map(|v| self.get_or_create_ir_type(v, hir, ir))
                .collect::<Result<Vec<_>, CodegenError>>()?;
            let return_type = self.get_or_create_ir_type(return_type, hir, ir)?;
            (args, return_type)
        };
        let Some(IRType::Function(id)) = self.get_mapped_type(&func_ty).map(|ty| ir.get_type(ty))
        else {
            unreachable!("The given function type should map to a valid function");
        };
        let ir_func = ir.get_function_type_mut(id);
        ir_func.insert_arg_types(&args);
        ir_func.set_return_type(return_type);
        Ok((args, return_type))
    }

    /// Lowers a single HIR function declaration into the IR.
    ///
    /// The lowering proceeds in three explicit stages, each isolated in its
    /// own private method:
    ///
    /// 1. **Entry setup** — creates the `$entry` label and initialises its
    ///    instruction-pointer bookkeeping on both the [`Function`] and the
    ///    [`Label`].  Argument types are queried once via the read-only
    ///    [`IRViewer`] API and cached in a [`FunctionContext`].
    ///
    /// 2. **Argument mapping** — inserts [`Value::new_func_arg`] entries for
    ///    each declared parameter and registers the HIR `VariableId` → IR
    ///    value mapping through [`TempIRData`].
    ///
    /// 3. **Body lowering** — delegates each [`HirStatement`] to the
    ///    pre-existing statement pipeline, which uses its own label tracking
    ///    via [`TempIRData::current_label`].
    ///
    /// [`IRViewer`]: slynx_ir::IRViewer
    /// [`TempIRData`]: crate::temporary_data::TempIRData
    /// [`TempIRData::current_label`]: crate::temporary_data::TempIRData::current_label
    pub(crate) fn initialize_function(
        &mut self,
        fptr: IRPointer<Function, 1>,
        func_ty: TypeId,
        statements: &[HirStatement],
        args: &[VariableId],
        hir: &SlynxHir,
        ir: &mut SlynxIR,
    ) -> Result<(), CodegenError> {
        let (arg_types, _) = self.mutate_function_type(func_ty, hir, ir)?;
        let builder = ir.build_function(fptr);

        let mut context = FunctionContext::new(builder);
        assert!(args.len() == arg_types.len());
        for (idx, (type_id, variable)) in arg_types.iter().zip(args).enumerate() {
            context.add_variable(*variable, Value::new_func_arg(idx, *type_id));
        }
        self.lower_body(&mut context, hir, statements);
        Ok(())
    }

    /// Phase 3 ── lowers the function body statement by statement.
    fn lower_body(
        &mut self,
        ctx: &mut FunctionContext,
        hir: &SlynxHir,
        statements: &[HirStatement],
    ) -> Result<(), CodegenError> {
        for statement in statements {
            self.lower_statement(statement, hir, ctx)?;
        }
        Ok(())
    }
}
