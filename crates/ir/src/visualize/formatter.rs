use std::collections::{BTreeSet, HashMap, HashSet};

use common::SymbolsModule;

use crate::{
    Component, Function, IRComponentId, IRPointer, IRSpecializedComponentType, IRType, IRTypes,
    Instruction, Label, Opcode, Operand, SlynxIR, Value,
};

pub struct Formatter<'a> {
    pub ir: &'a SlynxIR,
    pub labels: &'a [Label],
    pub functions: &'a [Function],
    pub components: &'a [Component],
    pub instructions: &'a [Instruction],
    pub types: &'a IRTypes,
    pub symbols: &'a SymbolsModule<SlynxIR>,
    label_offset: usize,
    inline_set: HashSet<usize>,
}

impl<'a> Formatter<'a> {
    pub fn new(ir: &'a SlynxIR) -> Self {
        Self {
            ir,
            labels: &ir.labels,
            functions: &ir.functions,
            components: &ir.components,
            instructions: &ir.instructions,
            types: &ir.types,
            symbols: &ir.strings,
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

    // ── type formatting ──

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
            IRType::GenericComponent => "anycomponent".to_string(),
            IRType::Struct(t) => self.fmt_struct_type(t),
            IRType::Component(c) => self.fmt_component_type(c),
            IRType::Specialized(IRSpecializedComponentType::Div) => "@div".to_string(),
            IRType::Specialized(IRSpecializedComponentType::Text) => "@text".to_string(),
        }
    }

    fn fmt_struct_type(&self, id: &crate::IRStructId) -> String {
        let strukt = self.types.get_object_type(*id);
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

    fn fmt_component_type(&self, id: &IRComponentId) -> String {
        let component = self.types.get_component_type(*id);
        format!("%{}", self.symbols.get_name(component.name()))
    }

    // ── top-level ──

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
                "struct %{}{{{fields}}};\n",
                self.symbols.get_name(name),
            ));
        }
        out
    }

    pub fn format_functions(&self) -> String {
        let mut out = Vec::new();
        for func in self.functions {
            out.push(self.format_function(func));
        }
        for component in self.components {
            out.push(self.format_component(component));
        }
        out.join("\n")
    }

    fn format_function(&self, func: &Function) -> String {
        let IRType::Function(fty) = self.types.get_type(func.ty()) else {
            unreachable!("Type of function should be function");
        };
        let func_ty = self.types.get_function_type(fty);
        let args = func_ty
            .get_args()
            .iter()
            .map(|ty| self.fmt_type(&self.types.get_type(*ty)))
            .collect::<Vec<_>>()
            .join(", ");
        let ret_ty = self.fmt_type(
            &self
                .types
                .get_type(self.types.get_function_type(fty).get_return_type()),
        );
        let mut out = format!(
            "{ret_ty} {}({args}){{\n",
            self.symbols.get_name(func.name())
        );

        let labels_ptr = func.labels_ptr();
        let fmt = self.with_label_offset(labels_ptr.ptr());
        for label in self.labels[labels_ptr.range()].iter() {
            out.push_str(&fmt.format_label(label));
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
            "component %{}({params}) {{\n",
            self.symbols.get_name(comp_ty.name()),
        );

        // UI instructions
        let ui_range = component.ui_instruction;
        for i in 0..ui_range.len() {
            let inst = &self.instructions[ui_range.ptr() + i];
            out.push_str("  ");
            out.push_str(&self.format_instruction(inst));
            out.push('\n');
        }

        out.push_str("}\n");
        out
    }

    // ── label formatting ──

    pub fn format_label(&self, label: &Label) -> String {
        let label_name = label.name();
        let label_name = self.ir.get_name(label_name);
        let header = if label.arguments().is_empty() {
            format!("${label_name}:\n")
        } else {
            let params = label
                .arguments()
                .iter()
                .enumerate()
                .map(|(i, _)| format!("lp{}", i))
                .collect::<Vec<_>>()
                .join(", ");
            format!("${label_name}({params}):\n")
        };

        let inline_set = self.build_inline_set(label);
        let fmt = Formatter {
            inline_set,
            ..*self
        };

        let range = label.instruction_range();
        let mut emitted = BTreeSet::new();
        let mut body = String::new();

        for real_idx in range.clone() {
            let mut deps = BTreeSet::new();
            fmt.collect_unmapped_deps(real_idx, &mut deps);
            for dep_idx in deps {
                // Skip deps that are within the label's own range — they
                // are already printed as main instructions.
                if range.contains(&dep_idx) {
                    continue;
                }
                if emitted.insert(dep_idx) {
                    body.push_str(&format!(
                        "  %t{dep_idx} = {}\n",
                        fmt.format_instruction(&fmt.instructions[dep_idx])
                    ));
                }
            }

            let instr = &fmt.instructions[real_idx];
            let line = fmt.format_instruction(instr);
            body.push_str("  ");
            if fmt.produces_value(instr) {
                if let Opcode::Allocate = &instr.opcode {
                    // ${slot_idx}
                    body.push_str(&format!("${real_idx} = {line}"));
                } else {
                    body.push_str(&format!("%t{real_idx} = {line}"));
                }
            } else {
                body.push_str(&line);
            }
            body.push('\n');
        }

        format!("{header}{body}")
    }

    // ── instruction formatting ──

    pub fn format_instruction(&self, instr: &Instruction) -> String {
        match &instr.opcode {
            Opcode::Br(label_ptr) => {
                let label_str = self.fmt_label_ref(*label_ptr);
                let args = self.fmt_operands(&instr.operands);
                if args.is_empty() {
                    format!("br {label_str};")
                } else {
                    format!("br {label_str}({args});")
                }
            }
            Opcode::Cbr {
                then_label,
                else_label,
            } => {
                let cond = self.fmt_value(instr.operands[0]);
                let then_str = self.fmt_label_ref(*then_label);
                let else_str = self.fmt_label_ref(*else_label);
                // Separate then_args / else_args from the flat operand list.
                // Operands: [cond, ...then_args, ...else_args]
                // We approximate; the precise split depends on label argument counts.
                // For simplicity, just show condition and labels.
                format!("cbr {cond}, {then_str}, {else_str};")
            }
            Opcode::Ret => {
                format!("ret {};", self.fmt_value(instr.operands[0]))
            }

            Opcode::Add => self.fmt_binary("add", instr),
            Opcode::Sub => self.fmt_binary("sub", instr),
            Opcode::Mul => self.fmt_binary("mul", instr),
            Opcode::Div => self.fmt_binary("div", instr),
            Opcode::Cmp => self.fmt_binary("cmp", instr),
            Opcode::Gt => self.fmt_binary("cmpgt", instr),
            Opcode::Gte => self.fmt_binary("cmpgte", instr),
            Opcode::Lt => self.fmt_binary("cmplt", instr),
            Opcode::Lte => self.fmt_binary("cmplte", instr),
            Opcode::And => self.fmt_binary("band", instr),
            Opcode::Or => self.fmt_binary("bor", instr),
            Opcode::Xor => self.fmt_binary("bxor", instr),
            Opcode::Shl => self.fmt_binary("shl", instr),
            Opcode::Shr => self.fmt_binary("shr", instr),
            Opcode::AShr => self.fmt_binary("ashr", instr),

            Opcode::GetField(index) => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                let target = self.fmt_value(instr.operands[0]);
                format!("getfield {ty_str}, {target}, {index};")
            }
            Opcode::SetField(index) => {
                let target = self.fmt_value(instr.operands[0]);
                let value = self.fmt_value(instr.operands[1]);
                format!("propset {target}, {index}, {value};")
            }
            Opcode::Call(func) => {
                let args = self.fmt_operands(&instr.operands);
                let view = self.ir.get_view(*func);
                let name = view.get_name();
                format!("{name}({args})")
            }
            Opcode::Allocate => {
                format!(
                    "allocate {};",
                    self.fmt_type(&self.types.get_type(instr.value_type))
                )
            }
            Opcode::Write => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "write {ty_str}, {}, {};",
                    self.fmt_value(instr.operands[0]),
                    self.fmt_value(instr.operands[1])
                )
            }
            Opcode::Read => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!("read {ty_str}, {};", self.fmt_value(instr.operands[0]))
            }
            Opcode::Reinterpret => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                format!(
                    "reinterpret {ty_str}, {};",
                    self.fmt_value(instr.operands[0])
                )
            }
            Opcode::Const(op) => self.fmt_operand(op),
            Opcode::RawValue => {
                if !instr.operands.is_empty() {
                    self.fmt_value(instr.operands[0])
                } else {
                    String::new()
                }
            }
            Opcode::Arg(idx) => {
                format!("arg {idx}")
            }
            Opcode::BlockParam(idx) => {
                format!("lp{idx}")
            }
            Opcode::SApply { property_code } => {
                let name = property_code.to_string();
                let component = self.fmt_value(instr.operands[0]);
                let value = self.fmt_value(instr.operands[1]);
                format!("@sapply {name}, {component}, {value};")
            }
            Opcode::InitCall(func) => {
                let comp = self.fmt_operands(&instr.operands);
                let view = self.ir.get_view(*func);
                let name = view.get_name();
                if instr.operands.len() <= 1 {
                    format!("@initcall {name}, {comp};")
                } else {
                    // The second operand is the style struct
                    format!("@initcall {name}, {comp};")
                }
            }
            Opcode::Struct | Opcode::Component => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                let args = self.fmt_operands(&instr.operands);
                format!("{ty_str}{{{args}}};")
            }
        }
    }

    // ── value formatting helpers ──

    fn fmt_value(&self, v: Value) -> String {
        if v.is_void() {
            return "void".to_string();
        }
        let instr = &self.instructions[v.idx()];
        match &instr.opcode {
            Opcode::Arg(n) => format!("p{n}"),
            Opcode::BlockParam(n) => format!("lp{n}"),
            Opcode::Const(op) => self.fmt_operand(op),
            Opcode::RawValue => {
                // raw_value has one operand which is a Const
                if !instr.operands.is_empty() {
                    self.fmt_value(instr.operands[0])
                } else {
                    format!("%t{}", v.idx())
                }
            }
            _ => {
                if self.inline_set.contains(&v.idx())
                    && matches!(&instr.opcode, Opcode::Component | Opcode::Struct)
                {
                    self.format_instruction(instr)
                        .trim_end_matches(';')
                        .to_string()
                } else {
                    format!("%t{}", v.idx())
                }
            }
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

    fn fmt_operands(&self, ops: &[Value]) -> String {
        ops.iter()
            .map(|&v| self.fmt_value(v))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn fmt_binary(&self, op: &str, instr: &Instruction) -> String {
        let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
        let a = self.fmt_value(instr.operands[0]);
        let b = self.fmt_value(instr.operands[1]);
        format!("{} {}, {}, {};", op, ty_str, a, b)
    }

    fn fmt_label_ref(&self, ptr: IRPointer<Label, 1>) -> String {
        let name_ptr = self.labels[ptr.ptr()].name();
        let name = self.symbols.get_name(name_ptr);
        format!("${name}")
    }

    // ── inline / unmapped dep helpers ──

    fn produces_value(&self, instr: &Instruction) -> bool {
        !matches!(
            instr.opcode,
            Opcode::Br(_) | Opcode::Cbr { .. } | Opcode::Write | Opcode::SetField(_) | Opcode::Ret
        )
    }

    fn count_refs(&self, instr_idx: usize, counts: &mut HashMap<usize, usize>) {
        let instr = &self.instructions[instr_idx];
        for &op_val in &instr.operands {
            if op_val.is_void() {
                continue;
            }
            let dep = op_val.idx();
            if matches!(
                self.instructions[dep].opcode,
                Opcode::Const(_) | Opcode::RawValue
            ) {
                continue;
            }
            *counts.entry(dep).or_insert(0) += 1;
            if *counts.get(&dep).unwrap() == 1 {
                self.count_refs(dep, counts);
            }
        }
    }

    fn build_inline_set(&self, label: &Label) -> HashSet<usize> {
        let range = label.instruction_range();
        let mut counts = HashMap::new();
        for idx in range {
            self.count_refs(idx, &mut counts);
        }
        counts
            .into_iter()
            .filter(|(idx, count)| {
                *count == 1
                    && matches!(
                        self.instructions[*idx].opcode,
                        Opcode::Component | Opcode::Struct
                    )
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn collect_unmapped_deps(&self, instr_idx: usize, out: &mut BTreeSet<usize>) {
        let instr = &self.instructions[instr_idx];
        for &op_val in &instr.operands {
            if op_val.is_void() {
                continue;
            }
            let dep_idx = op_val.idx();
            if matches!(
                self.instructions[dep_idx].opcode,
                Opcode::Const(_) | Opcode::RawValue
            ) {
                continue;
            }
            if self.inline_set.contains(&dep_idx) {
                continue;
            }
            if out.insert(dep_idx) {
                self.collect_unmapped_deps(dep_idx, out);
            }
        }
    }
}
