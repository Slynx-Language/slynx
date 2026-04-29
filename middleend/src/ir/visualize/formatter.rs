use common::SymbolsModule;

use crate::{
    Context, IRComponentId, IRPointer, IRSpecializedComponentType, IRStructId, IRType, IRTypeId,
    IRTypes, Instruction, InstructionType, Label, Operand, Slot, Value,
};

/// Formatter holds the slices and helpers necessary to render the IR in textual form (SIR).
/// It provides instance methods that replace the free functions previously living in the
/// `visualize` module.
pub struct Formatter<'a> {
    pub labels: &'a [Label],
    pub contexts: &'a [Context],
    pub values: &'a [Value],
    pub operands: &'a [Operand],
    pub types: &'a IRTypes,
    pub symbols: &'a SymbolsModule,
}

impl<'a> Formatter<'a> {
    /// Create a new formatter instance.
    pub fn new(
        labels: &'a [Label],
        contexts: &'a [Context],
        values: &'a [Value],
        operands: &'a [Operand],
        types: &'a IRTypes,
        symbols: &'a SymbolsModule,
    ) -> Self {
        Self {
            contexts,
            labels,
            values,
            operands,
            types,
            symbols,
        }
    }

    fn fmt_struct(&self, t: &IRStructId) -> String {
        let strukt = self.types.get_object_type(*t);
        if let Some(name) = strukt.name() {
            format!("%{}", self.symbols.get_name(name))
        } else {
            let fields = self
                .types
                .get_object_type(*t)
                .get_fields()
                .iter()
                .map(|v| {
                    let t = self.types.get_type(*v);
                    self.fmt_type(&t)
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }

    fn fmt_component(&self, t: &IRComponentId) -> String {
        let component = self.types.get_component_type(*t);
        format!("%{}", self.symbols.get_name(component.name()))
    }
    /// Convert IRType into a human friendly string. Kept as a free fn for reuse inside the module.
    fn fmt_type(&self, ty: &IRType) -> String {
        match ty {
            IRType::I8 => "i8".to_string(),
            IRType::U8 => "u8".to_string(),
            IRType::I16 => "i16".to_string(),
            IRType::U16 => "u16".to_string(),
            IRType::I32 => "i32".to_string(),
            IRType::U32 => "u32".to_string(),
            IRType::I64 => "i64".to_string(),
            IRType::U64 => "u64".to_string(),
            IRType::F32 => "f32".to_string(),
            IRType::F64 => "f64".to_string(),
            IRType::BOOL => "bool".to_string(),
            IRType::VOID => "void".to_string(),
            IRType::STR => "str".to_string(),
            IRType::Struct(t) => self.fmt_struct(t),
            IRType::Component(c) => self.fmt_component(c),
            IRType::Function(_) => "fn".to_string(),
            IRType::GenericComponent => "component".to_string(),
            IRType::ISIZE => "isize".to_string(),
            IRType::USIZE => "usize".to_string(),
            IRType::Specialized(IRSpecializedComponentType::Div) => "specialized(div)".to_string(),
            IRType::Specialized(IRSpecializedComponentType::Text) => {
                "specialized(text)".to_string()
            }
        }
    }

    pub fn format_contexts(&self, instructions: &[Instruction]) -> String {
        let mut out = Vec::new();
        for ctx in self.contexts {
            let ty = ctx.ty();
            let IRType::Function(fty) = self.types.get_type(ty) else {
                unreachable!("Type of context should be function");
            };
            let ty = self.types.get_function_type(fty);
            let ty = self.fmt_type(&self.types.get_type(ty.get_return_type()));
            let mut current = format!("{ty} {}(){{\n", self.symbols.get_name(ctx.name()));
            let range = ctx.labels_ptr().range();
            let mut idx = 0;
            for label in &self.labels[range.start..range.end] {
                current.push_str(&self.format_label(&label, idx, instructions));
                idx += 1;
            }
            current.push_str("}\n");
            out.push(current);
        }
        out.join("\n")
    }

    pub fn format_types(&self) -> String {
        let mut out = String::new();
        for (name, fields) in self
            .types
            .structs()
            .iter()
            .filter_map(|s| s.name().map(|name| (name, s.get_fields())))
        {
            let fields = fields
                .iter()
                .map(|f| self.fmt_type(&self.types.get_type(*f)))
                .collect::<Vec<_>>()
                .join(",");
            let v = format!("struct %{}{{{}}};\n", self.symbols.get_name(name), fields);
            out.push_str(&v);
        }
        out
    }

    pub fn format_label(&self, label: &Label, idx: usize, instructions: &[Instruction]) -> String {
        let mut out = String::new();

        if label.arguments().is_empty() {
            out.push_str(&format!("$label_{}:\n", idx));
        } else {
            let params = label
                .arguments()
                .iter()
                .enumerate()
                .map(|(i, _)| format!("lp{}", i))
                .collect::<Vec<_>>()
                .join(", ");

            out.push_str(&format!("$label_{}({}):\n", idx, params));
        }

        // Instructions inside the label
        let iptr = label.instructions();
        for i in 0..iptr.len() {
            let instr_ptr = iptr.ptr_to(i);
            let instr_idx = instr_ptr.ptr();
            let instr = &instructions[instr_idx];

            let fmt = Formatter::new(
                self.labels,
                self.contexts,
                self.values,
                self.operands,
                self.types,
                self.symbols,
            );
            let line = fmt.format_instruction(instr);

            let produces_value = !matches!(
                instr.instruction_type,
                InstructionType::Br(_)
                    | InstructionType::Cbr { .. }
                    | InstructionType::Write(_)
                    | InstructionType::Ret
            );

            out.push_str("  ");

            if produces_value {
                out.push_str(&format!("%t{} = {}", instr_idx, line));
            } else {
                out.push_str(&line);
            }

            out.push('\n');
        }

        out
    }

    pub fn format_labels(&self, instructions: &[Instruction]) -> String {
        let mut out = String::new();
        for (i, label) in self.labels.iter().enumerate() {
            out.push_str(&self.format_label(label, i, instructions));
            out.push('\n');
        }
        out
    }
    /// Format a single instruction into a String (the full textual representation).
    pub fn format_instruction(&self, instr: &Instruction) -> String {
        match &instr.instruction_type {
            InstructionType::Br(label_ptr) => {
                let idx = label_ptr.ptr();
                let len = instr.operands.len();

                if len == 0 {
                    format!("br $label_{};", idx)
                } else {
                    let args = (0..len)
                        .map(|i| self.fmt_value(&instr.operands.ptr_to(i)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if args.len() > 0 {
                        format!("br $label_{}({});", idx, args)
                    } else {
                        format!("br $label_{};", idx)
                    }
                }
            }

            InstructionType::Cbr {
                then_label,
                else_label,
                then_args,
                else_args,
            } => {
                let cond = self.fmt_value(&instr.operands.ptr_to(0));
                let then_str = self.fmt_label_ref(then_label);
                let else_str = self.fmt_label_ref(else_label);
                let then_a = self.fmt_value_range(then_args);
                let else_a = self.fmt_value_range(else_args);
                format!(
                    "cbr {}, {}({}), {}({});",
                    cond, then_str, then_a, else_str, else_a
                )
            }

            InstructionType::Ret => {
                let val = self.fmt_value(&instr.operands.ptr_to(0));
                format!("ret {};", val)
            }

            InstructionType::Add => self.fmt_binary("add", instr),
            InstructionType::Sub => self.fmt_binary("sub", instr),
            InstructionType::Mul => self.fmt_binary("mul", instr),
            InstructionType::Div => self.fmt_binary("div", instr),
            InstructionType::Cmp => self.fmt_binary("cmp", instr),
            InstructionType::Gt => self.fmt_binary("cmpgt", instr),
            InstructionType::Gte => self.fmt_binary("cmpgte", instr),
            InstructionType::Lt => self.fmt_binary("cmplt", instr),
            InstructionType::Lte => self.fmt_binary("cmplte", instr),
            InstructionType::And => self.fmt_binary("band", instr),
            InstructionType::Or => self.fmt_binary("bor", instr),
            InstructionType::Xor => self.fmt_binary("bxor", instr),
            InstructionType::Shl => self.fmt_binary("shl", instr),
            InstructionType::Shr => self.fmt_binary("shr", instr),
            InstructionType::AShr => self.fmt_binary("ashr", instr),

            InstructionType::GetField(u) => self.fmt_get_field(instr, *u),
            InstructionType::SetField(u) => self.fmt_set_field(instr, *u),
            InstructionType::FunctionCall(temp) => self.fmt_function_call(instr, temp),

            InstructionType::Allocate(_) => self.fmt_allocate(instr),
            InstructionType::Write(slot) => self.fmt_write(instr, slot),
            InstructionType::Read => self.fmt_read(instr),
            InstructionType::Reinterpret => self.fmt_reinterpret(instr),

            InstructionType::RawValue => {
                let ptr = instr.operands.ptr_to(0);
                self.fmt_value(&ptr)
            }
            InstructionType::Struct | InstructionType::Component => {
                "Struct/Component not implemented".to_string()
            }
        }
    }

    #[inline]
    fn fmt_label_ref(&self, ptr: &IRPointer<Label, 1>) -> String {
        let idx = ptr.ptr();
        format!("$label_{}", idx)
    }

    fn fmt_get_field(&self, instr: &Instruction, index: usize) -> String {
        let target = self.fmt_value(&instr.operands.ptr_to(0));
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        format!("getfield {}, {}, {};", ty_str, target, index)
    }

    fn fmt_set_field(&self, instr: &Instruction, slot: usize) -> String {
        let target = self.fmt_value(&instr.operands.ptr_to(0));
        // slot here is an index into struct fields (not an IRPointer)
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        format!("setfield {}, {}, {};", ty_str, target, slot)
    }

    fn fmt_function_call(&self, instr: &Instruction, func: &IRPointer<Context, 1>) -> String {
        let args = (0..instr.operands.len())
            .map(|i| self.fmt_value(&instr.operands.ptr_to(i)))
            .collect::<Vec<_>>()
            .join(", ");

        let fname = format!("f{}", func.ptr());

        format!("call {}, {};", fname, args)
    }

    fn fmt_allocate(&self, instr: &Instruction) -> String {
        format!(
            "allocate {};",
            self.fmt_type(&self.types.get_type(instr.value_type))
        )
    }

    fn fmt_write(&self, instr: &Instruction, slot: &IRPointer<Slot, 1>) -> String {
        let value = self.fmt_value(&instr.operands.ptr_to(0));
        let slot_str = format!("${}", slot.ptr());
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        format!("write {}, {}, {};", ty_str, slot_str, value)
    }

    fn fmt_read(&self, instr: &Instruction) -> String {
        let slot = instr.operands.ptr_to(0);
        let slot_str = self.fmt_value(&slot);
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        format!("read {}, {};", ty_str, slot_str)
    }

    fn fmt_reinterpret(&self, instr: &Instruction) -> String {
        let slot_str = self.fmt_value(&instr.operands.ptr_to(0));
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        format!("reinterpret {}, {};", ty_str, slot_str)
    }

    fn fmt_value(&self, ptr: &IRPointer<Value, 1>) -> String {
        match &self.values[ptr.ptr()] {
            Value::FuncArg(n) => format!("p{}", n),
            Value::LabelArg(n) => format!("lp{}", n),
            Value::Instruction(p) => format!("%t{}", p.ptr()),
            Value::Slot(p) => format!("${}", p.ptr()),
            Value::Raw(op_ptr) => {
                let op = &self.operands[op_ptr.ptr()];
                match op {
                    Operand::Bool(b) => b.to_string(),
                    Operand::Int(i) => i.to_string(),
                    Operand::Float(f) => f.to_string(),
                    Operand::String(sym) => format!("\"{}\"", self.symbols.get_name(*sym)),
                }
            }
            Value::StructLiteral(_, _) => format!("%lit{}", ptr.ptr()),
            Value::Void => "void".to_string(),
            Value::Specliazed(_) => "Specialized not implemented".to_string(),
        }
    }

    fn fmt_value_range(&self, ptr: &IRPointer<Value>) -> String {
        (0..ptr.len())
            .map(|i| self.fmt_value(&ptr.ptr_to(i)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn fmt_binary(&self, op: &str, instr: &Instruction) -> String {
        let a = self.fmt_value(&instr.operands.ptr_to(0));
        let b = self.fmt_value(&instr.operands.ptr_to(1));
        let sla = self.types.get_type(instr.value_type);
        let ty_str = self.fmt_type(&sla);
        format!("{} {}, {}, {};", op, ty_str, a, b)
    }
}
