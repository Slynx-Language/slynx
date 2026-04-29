use std::collections::{BTreeSet, HashMap, HashSet};

use common::SymbolsModule;

use crate::{
    Component, Context, IRComponentId, IRPointer, IRSpecializedComponent,
    IRSpecializedComponentType, IRStructId, IRType, IRTypes, Instruction, InstructionType, Label,
    Operand, SlynxIR, Value,
};

pub struct Formatter<'a> {
    pub ir: &'a SlynxIR,
    pub labels: &'a [Label],
    pub contexts: &'a [Context],
    pub components: &'a [Component],
    pub values: &'a [Value],
    pub operands: &'a [Operand],
    pub types: &'a IRTypes,
    pub symbols: &'a SymbolsModule,
    pub specialized: &'a [IRSpecializedComponent],
    instruction_pointers: &'a [IRPointer<Instruction>],
    instructions: &'a [Instruction],
    label_offset: usize,
    inline_set: HashSet<usize>,
}

impl<'a> Formatter<'a> {
    pub fn new(ir: &'a SlynxIR, symbols: &'a SymbolsModule) -> Self {
        let labels = &ir.labels;
        let contexts = &ir.contexts;
        let components = &ir.components;
        let values = &ir.values;
        let operands = &ir.operands;
        let types = &ir.types;
        let specialized = &ir.specialized;
        let instruction_pointers = &ir.instruction_pointers;
        let instructions = &ir.instructions;
        Self {
            ir,
            contexts,
            components,
            labels,
            values,
            operands,
            types,
            symbols,
            specialized,
            instruction_pointers,
            instructions,
            label_offset: 0,
            inline_set: HashSet::new(),
        }
    }

    fn with_label_offset(&self, offset: usize) -> Formatter<'a> {
        Formatter {
            label_offset: offset,
            inline_set: HashSet::new(),
            ..*self
        }
    }

    // --- type formatting ---

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
            IRType::ISIZE => "isize".to_string(),
            IRType::USIZE => "usize".to_string(),
            IRType::Function(_) => "fn".to_string(),
            IRType::GenericComponent => "component".to_string(),
            IRType::Struct(t) => self.fmt_struct_type(t),
            IRType::Component(c) => self.fmt_component_type(c),
            IRType::Specialized(IRSpecializedComponentType::Div) => "specialized(div)".to_string(),
            IRType::Specialized(IRSpecializedComponentType::Text) => {
                "specialized(text)".to_string()
            }
        }
    }

    fn fmt_struct_type(&self, t: &IRStructId) -> String {
        let strukt = self.types.get_object_type(*t);
        if let Some(name) = strukt.name() {
            format!("%{}", self.symbols.get_name(name))
        } else {
            let fields = strukt
                .get_fields()
                .iter()
                .map(|v| self.fmt_type(&self.types.get_type(*v)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }

    fn fmt_component_type(&self, t: &IRComponentId) -> String {
        let component = self.types.get_component_type(*t);
        format!("%{}", self.symbols.get_name(component.name()))
    }

    // --- top-level formatting ---

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
            out.push_str(&format!(
                "struct %{}{{{}}};\n",
                self.symbols.get_name(name),
                fields
            ));
        }
        out
    }

    pub fn format_contexts(&self) -> String {
        let mut out = Vec::new();
        for ctx in self.contexts {
            out.push(self.format_function(ctx));
        }
        for component in self.components {
            out.push(self.format_component(component));
        }
        out.join("\n")
    }

    fn format_function(&self, ctx: &Context) -> String {
        let IRType::Function(fty) = self.types.get_type(ctx.ty()) else {
            unreachable!("Type of context should be function");
        };
        let ret_ty = self.fmt_type(
            &self
                .types
                .get_type(self.types.get_function_type(fty).get_return_type()),
        );
        let mut out = format!("{ret_ty} {}(){{\n", self.symbols.get_name(ctx.name()));

        let labels_ptr = ctx.labels_ptr();
        let fmt = self.with_label_offset(labels_ptr.ptr());
        for (idx, label) in self.labels[labels_ptr.range()].iter().enumerate() {
            out.push_str(&fmt.format_label(label, idx));
        }
        out.push_str("}\n");
        out
    }

    fn format_component(&self, component: &Component) -> String {
        let IRType::Component(cid) = self.types.get_type(component.ir_type()) else {
            unreachable!("Type of component should be Component");
        };
        let comp_ty = self.types.get_component_type(cid);
        let params = comp_ty
            .fields()
            .iter()
            .map(|v| self.fmt_type(&self.types.get_type(*v)))
            .collect::<Vec<_>>()
            .join(", ");

        let mut out = format!(
            "component %{}({}) {{\n",
            self.symbols.get_name(comp_ty.name()),
            params
        );

        for (i, field_ty) in comp_ty.fields().iter().enumerate() {
            out.push_str(&format!(
                "  %field{}: {} = p{};\n",
                i,
                self.fmt_type(&self.types.get_type(*field_ty)),
                i
            ));
        }

        let values_range = component.values();
        for i in 0..values_range.len() {
            let child_str = match &self.values[values_range.ptr_to(i).ptr()] {
                Value::Instruction(p) => format!("#t{}: component %t{}", i, p.ptr()),
                Value::Specliazed(spec_ptr) => {
                    let kind = match &self.specialized[spec_ptr.ptr()] {
                        IRSpecializedComponent::Text(_) => "specialized(text)",
                        IRSpecializedComponent::Div(_) => "specialized(div)",
                    };
                    format!("#t{}: {}", i, kind)
                }
                _ => format!("#t{}: unknown", i),
            };
            out.push_str(&format!("  {};\n", child_str));
        }

        out.push_str("}\n");
        out
    }

    // --- label formatting ---

    pub fn format_label(&self, label: &Label, idx: usize) -> String {
        let header = if label.arguments().is_empty() {
            format!("$label_{}:\n", idx)
        } else {
            let params = label
                .arguments()
                .iter()
                .enumerate()
                .map(|(i, _)| format!("lp{}", i))
                .collect::<Vec<_>>()
                .join(", ");
            format!("$label_{}({}):\n", idx, params)
        };

        let inline_set = self.build_inline_set(label);
        let fmt = Formatter {
            inline_set,
            ..*self
        };

        let iptr = label.instructions();
        let mut emitted = BTreeSet::new();
        let mut body = String::new();

        for i in 0..iptr.len() {
            let real_idx = fmt.instruction_pointers[iptr.ptr_to(i).ptr()].ptr();

            let mut deps = BTreeSet::new();
            fmt.collect_unmapped_deps(real_idx, &mut deps);
            for dep_idx in deps {
                if emitted.insert(dep_idx) {
                    body.push_str(&format!(
                        "  %t{} = {}\n",
                        dep_idx,
                        fmt.format_instruction(&fmt.instructions[dep_idx])
                    ));
                }
            }

            let instr = &fmt.instructions[real_idx];
            let line = fmt.format_instruction(instr);
            body.push_str("  ");
            if fmt.produces_value(instr) {
                let lhs = if let InstructionType::Allocate(slot) = &instr.instruction_type {
                    format!("${}", slot.ptr())
                } else {
                    format!("%t{}", real_idx)
                };
                body.push_str(&format!("{} = {}", lhs, line));
            } else {
                body.push_str(&line);
            }
            body.push('\n');
        }

        format!("{}{}", header, body)
    }

    pub fn format_labels(&self) -> String {
        self.labels
            .iter()
            .enumerate()
            .map(|(i, label)| format!("{}\n", self.format_label(label, i)))
            .collect()
    }

    // --- instruction formatting ---

    pub fn format_instruction(&self, instr: &Instruction) -> String {
        match &instr.instruction_type {
            InstructionType::Br(label_ptr) => {
                let label_str = self.fmt_label_ref(label_ptr);
                let args = self.fmt_operands_range(0, instr.operands.len(), &instr.operands);
                if args.is_empty() {
                    format!("br {};", label_str)
                } else {
                    format!("br {}({});", label_str, args)
                }
            }
            InstructionType::Cbr {
                then_label,
                else_label,
                then_args,
                else_args,
            } => {
                let cond = self.fmt_value(&instr.operands.ptr_to(0));
                format!(
                    "cbr {}, {}({}), {}({});",
                    cond,
                    self.fmt_label_ref(then_label),
                    self.fmt_value_range(then_args),
                    self.fmt_label_ref(else_label),
                    self.fmt_value_range(else_args),
                )
            }
            InstructionType::Ret => format!("ret {};", self.fmt_value(&instr.operands.ptr_to(0))),

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

            InstructionType::GetField(u) => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "getfield {}, {}, {};",
                    ty_str,
                    self.fmt_value(&instr.operands.ptr_to(0)),
                    u
                )
            }
            InstructionType::SetField(u) => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "setfield {}, {}, {};",
                    ty_str,
                    self.fmt_value(&instr.operands.ptr_to(0)),
                    u
                )
            }
            InstructionType::FunctionCall(func) => {
                let args = self.fmt_operands_range(0, instr.operands.len(), &instr.operands);
                format!("call f{}, {};", func.ptr(), args)
            }
            InstructionType::Allocate(_) => {
                format!(
                    "allocate {};",
                    self.fmt_type(&self.types.get_type(instr.value_type))
                )
            }
            InstructionType::Write(slot) => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "write {}, ${}, {};",
                    ty_str,
                    slot.ptr(),
                    self.fmt_value(&instr.operands.ptr_to(0))
                )
            }
            InstructionType::Read => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "read {}, {};",
                    ty_str,
                    self.fmt_value(&instr.operands.ptr_to(0))
                )
            }
            InstructionType::Reinterpret => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "reinterpret {}, {};",
                    ty_str,
                    self.fmt_value(&instr.operands.ptr_to(0))
                )
            }
            InstructionType::RawValue => self.fmt_value(&instr.operands.ptr_to(0)),
            InstructionType::Struct | InstructionType::Component => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                let args = self.fmt_operands_range(0, instr.operands.len(), &instr.operands);
                format!("{}({});", ty_str, args)
            }
        }
    }

    // --- value / operand helpers ---

    fn fmt_value(&self, ptr: &IRPointer<Value, 1>) -> String {
        match &self.values[ptr.ptr()] {
            Value::FuncArg(n) => format!("p{}", n),
            Value::LabelArg(n) => format!("lp{}", n),
            Value::Slot(p) => format!("${}", p.ptr()),
            Value::Raw(op_ptr) => self.fmt_operand(&self.operands[op_ptr.ptr()]),
            Value::Instruction(p) => {
                let instr = &self.instructions[p.ptr()];
                if matches!(instr.instruction_type, InstructionType::RawValue) {
                    self.fmt_value(&instr.operands.ptr_to(0))
                } else if self.inline_set.contains(&p.ptr()) {
                    self.format_instruction(instr)
                        .trim_end_matches(';')
                        .to_string()
                } else {
                    format!("%t{}", p.ptr())
                }
            }
            Value::StructLiteral(_, _) => format!("%lit{}", ptr.ptr()),
            Value::Void => "void".to_string(),
            Value::Specliazed(_) => "specialized".to_string(),
        }
    }

    fn fmt_operand(&self, op: &Operand) -> String {
        match op {
            Operand::Bool(b) => b.to_string(),
            Operand::Int(i) => i.to_string(),
            Operand::Float(f) => f.to_string(),
            Operand::String(sym) => format!("\"{}\"", self.symbols.get_name(*sym)),
        }
    }

    fn fmt_value_range(&self, ptr: &IRPointer<Value>) -> String {
        (0..ptr.len())
            .map(|i| self.fmt_value(&ptr.ptr_to(i)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn fmt_operands_range(&self, start: usize, len: usize, base: &IRPointer<Value>) -> String {
        (start..start + len)
            .map(|i| self.fmt_value(&base.ptr_to(i)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn fmt_binary(&self, op: &str, instr: &Instruction) -> String {
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        let a = self.fmt_value(&instr.operands.ptr_to(0));
        let b = self.fmt_value(&instr.operands.ptr_to(1));
        format!("{} {}, {}, {};", op, ty_str, a, b)
    }

    fn fmt_label_ref(&self, ptr: &IRPointer<Label, 1>) -> String {
        format!("$label_{}", ptr.ptr().saturating_sub(self.label_offset))
    }

    // --- inline / unmapped dep helpers ---

    fn is_mapped(&self, instr_idx: usize) -> bool {
        self.instruction_pointers
            .iter()
            .any(|p| p.ptr() == instr_idx)
    }

    fn is_inlinable(instr: &Instruction) -> bool {
        matches!(
            instr.instruction_type,
            InstructionType::Component | InstructionType::Struct
        )
    }

    fn produces_value(&self, instr: &Instruction) -> bool {
        !matches!(
            instr.instruction_type,
            InstructionType::Br(_)
                | InstructionType::Cbr { .. }
                | InstructionType::Write(_)
                | InstructionType::Ret
        )
    }

    fn count_refs(&self, instr_idx: usize, counts: &mut HashMap<usize, usize>) {
        let instr = &self.instructions[instr_idx];
        for i in 0..instr.operands.len() {
            if let Value::Instruction(p) = &self.values[instr.operands.ptr_to(i).ptr()] {
                let dep = p.ptr();
                if matches!(
                    self.instructions[dep].instruction_type,
                    InstructionType::RawValue
                ) {
                    continue;
                }
                if !self.is_mapped(dep) {
                    *counts.entry(dep).or_insert(0) += 1;
                    if *counts.get(&dep).unwrap() == 1 {
                        self.count_refs(dep, counts);
                    }
                }
            }
        }
    }

    fn build_inline_set(&self, label: &Label) -> HashSet<usize> {
        let iptr = label.instructions();
        let mut counts = HashMap::new();
        for i in 0..iptr.len() {
            let real_idx = self.instruction_pointers[iptr.ptr_to(i).ptr()].ptr();
            self.count_refs(real_idx, &mut counts);
        }
        counts
            .into_iter()
            .filter(|(idx, count)| *count == 1 && Self::is_inlinable(&self.instructions[*idx]))
            .map(|(idx, _)| idx)
            .collect()
    }

    fn collect_unmapped_deps(&self, instr_idx: usize, out: &mut BTreeSet<usize>) {
        let instr = &self.instructions[instr_idx];
        for i in 0..instr.operands.len() {
            if let Value::Instruction(p) = &self.values[instr.operands.ptr_to(i).ptr()] {
                let dep_idx = p.ptr();
                if matches!(
                    self.instructions[dep_idx].instruction_type,
                    InstructionType::RawValue
                ) {
                    continue;
                }
                if self.inline_set.contains(&dep_idx) {
                    continue;
                }
                if !self.is_mapped(dep_idx) && out.insert(dep_idx) {
                    self.collect_unmapped_deps(dep_idx, out);
                }
            }
        }
    }
}
