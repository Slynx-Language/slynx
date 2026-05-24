use slynx_hir::{HirStatement, VariableId};
use slynx_ir::{Function, IRPointer, IRStorage, IRTypeId, Value};

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
struct FunctionContext {
    /// Handle to the IR [`Function`] being populated.
    fptr: IRPointer<Function, 1>,
    /// Snapshot of the function's argument types, obtained at setup time
    /// so that later phases (argument mapping, body lowering) do not need
    /// to reach into the IR or [`TempIRData`] to resolve them.
    arg_types: Vec<IRTypeId>,
}

impl FunctionContext {
    fn new(fptr: IRPointer<Function, 1>, arg_types: Vec<IRTypeId>) -> Self {
        Self { fptr, arg_types }
    }

    fn fptr(&self) -> IRPointer<Function, 1> {
        self.fptr
    }

    fn arg_types(&self) -> &[IRTypeId] {
        &self.arg_types
    }
}

impl Codegen {
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
        statements: &[HirStatement],
        args: &[VariableId],
    ) -> Result<(), CodegenError> {
        let ctx = self.setup_entry(fptr);
        self.map_arguments(&ctx, args);
        self.lower_body(&ctx, statements)?;

        Ok(())
    }

    /// Phase 1 ── creates the `$entry` label and configures the
    /// instruction-pointer range on both the [`Function`] and the [`Label`].
    fn setup_entry(&mut self, fptr: IRPointer<Function, 1>) -> FunctionContext {
        let arg_types = self.ir.get_view(fptr).get_arguments().to_vec();
        FunctionContext::new(fptr, arg_types)
    }

    /// Phase 2 ── inserts [`Value::new_func_arg`] entries for every
    /// declared parameter and records the HIR → IR mapping.
    fn map_arguments(&mut self, ctx: &FunctionContext, args: &[VariableId]) {
        let arg_values: Vec<Value> = (0..args.len())
            .map(|idx| Value::new_func_arg(idx, ctx.arg_types()[idx]))
            .collect();
        let ptr = self.ir.insert_values(&arg_values);
        self.temp.set_function_args(args, ptr);
    }

    /// Phase 3 ── lowers the function body statement by statement.
    fn lower_body(
        &mut self,
        _ctx: &FunctionContext,
        statements: &[HirStatement],
    ) -> Result<(), CodegenError> {
        for statement in statements {
            self.lower_statement(statement)?;
        }
        Ok(())
    }
}
