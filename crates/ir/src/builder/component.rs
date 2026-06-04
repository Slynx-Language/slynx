use smallvec::smallvec;

use crate::{
    Component, Function, FunctionBuilder, IRPointer, IRStorage, IRType, IRTypeId, Instruction,
    Opcode, Operand, SlynxIR, Value,
};

#[derive(Debug)]
pub struct InitialCall {
    func_ptr: IRPointer<Function, 1>,
    args: Vec<Value>,
}

#[derive(Debug)]
pub struct ComponentBuilder<'a> {
    component_id: IRPointer<Component, 1>,
    fields: Vec<IRTypeId>,
    children: Vec<IRTypeId>,
    child_values: Vec<Value>,
    initial_calls: Vec<InitialCall>,
    preamble_start: Option<usize>,
    ir: &'a mut SlynxIR,
}

impl<'a> ComponentBuilder<'a> {
    pub fn new(component_id: IRPointer<Component, 1>, ir: &'a mut SlynxIR) -> Self {
        Self {
            component_id,
            ir,
            fields: Vec::new(),
            children: Vec::new(),
            child_values: Vec::new(),
            initial_calls: Vec::new(),
            preamble_start: None,
        }
    }

    pub fn emit_arg(&mut self, arg: u32) -> Value {
        let idx = self.ir.instructions.len() as u32;
        self.ir
            .instructions
            .push(Instruction::arg(arg, self.fields[arg as usize]));
        Value::instruction(idx)
    }

    /// Emit a `Component` instruction as a preamble to the initcalls,
    /// creating a temporary value of the given type.  The resulting
    /// [`Value`] can be passed to [`add_initial_call`].
    pub fn emit_temp_component(&mut self, ty: IRTypeId) -> Value {
        if self.preamble_start.is_none() {
            self.preamble_start = Some(self.ir.instructions.len());
        }
        let idx = self.ir.instructions.len() as u32;
        self.ir
            .instructions
            .push(Instruction::component(ty, smallvec![]));
        Value::instruction(idx)
    }

    /// Emit an `Opcode::Component` instruction for a child element and record its Value.
    /// The emitted instruction is NOT part of the preamble/initcall range — it is stored
    /// separately in `child_values` so the JS backend can emit `let cN = document.createElement(...)`.
    pub fn emit_child_component(&mut self, ty: IRTypeId) -> Value {
        let idx = self.ir.instructions.len() as u32;
        self.ir
            .instructions
            .push(Instruction::component(ty, smallvec![]));
        let val = Value::instruction(idx);
        self.child_values.push(val);
        val
    }

    pub fn emit_child(&mut self, index: u16) -> Value {
        if self.preamble_start.is_none() {
            self.preamble_start = Some(self.ir.instructions.len());
        }
        let idx = self.ir.instructions.len() as u32;
        let component = self.ir.get(self.component_id);
        self.ir
            .instructions
            .push(Instruction::getchild(index, component.ty));
        Value::instruction(idx)
    }

    /// Emit a compile-time constant instruction into the preamble.
    /// Returns the [`Value`] that can be passed to [`add_initial_call`].
    pub fn emit_const_placeholder(&mut self, op: Operand, ty: IRTypeId) -> Value {
        if self.preamble_start.is_none() {
            self.preamble_start = Some(self.ir.instructions.len());
        }
        let idx = self.ir.instructions.len() as u32;
        self.ir.instructions.push(Instruction::const_value(op, ty));
        Value::instruction(idx)
    }

    pub fn ir(&mut self) -> &mut SlynxIR {
        self.ir
    }

    pub fn add_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }

    pub fn add_child(&mut self, child: IRTypeId) {
        self.children.push(child);
    }

    pub fn add_initial_call(&mut self, f: IRPointer<Function, 1>, args: Vec<Value>) {
        self.initial_calls.push(InitialCall { func_ptr: f, args });
    }

    pub fn generate(mut self) -> IRPointer<Component, 1> {
        {
            let component = self.ir.get(self.component_id);
            let IRType::Component(ty) = self.ir.get_type(component.ty) else {
                unreachable!("Type of component should be, on IR, component");
            };
            let ty = self.ir.get_component_type_mut(ty);
            ty.fields.extend_from_slice(&self.fields);
            ty.children.extend_from_slice(&self.children);
        }

        // Store child Values on the Component (they are NOT part of ui_instruction)
        let comp = self.ir.get_mut(self.component_id);
        comp.child_values = std::mem::take(&mut self.child_values);

        let start = self.preamble_start.unwrap_or(self.ir.instructions.len());
        for initcall in self.initial_calls {
            self.ir.instructions.push(Instruction::initcall(
                initcall.func_ptr,
                initcall.args.into(),
                self.ir.void_type(),
            ));
        }
        let len = self.ir.instructions.len() - start;
        self.ir.get_mut(self.component_id).ui_instruction = IRPointer::new(start, len);
        self.component_id
    }
}

pub struct ComponentValueBuilder<'f, 'ir> {
    func: &'f mut FunctionBuilder<'ir>,
    ty: IRTypeId,
    args: Vec<Value>,
}

impl<'f, 'ir> ComponentValueBuilder<'f, 'ir> {
    pub fn new(function: &'f mut FunctionBuilder<'ir>, ty: IRTypeId) -> Self {
        Self {
            func: function,
            ty,
            args: Vec::new(),
        }
    }

    pub fn add_argument(&mut self, arg: Value) {
        self.args.push(arg);
    }

    pub fn generate(self) -> Value {
        self.func.emit(Opcode::Component, self.args, self.ty)
    }
}
