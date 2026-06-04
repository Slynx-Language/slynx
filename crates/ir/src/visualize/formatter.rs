use std::collections::{BTreeSet, HashMap, HashSet};

use common::SymbolsModule;

use crate::{
    Component, Function, IRComponentId, IRPointer, IRSpecializedComponentType, IRType, IRTypes,
    IRViewer, Instruction, Label, Opcode, Operand, SlynxIR, Value,
};

pub struct Formatter<'a> {
    pub ir: &'a SlynxIR,
    pub labels: &'a [Label],
    pub functions: &'a [Function],
    pub components: &'a [Component],
    pub instructions: &'a [Instruction],
    pub types: &'a IRTypes,
    pub symbols: &'a SymbolsModule<SlynxIR>,
    inline_set: HashSet<usize>,
    /// Maps instruction index → variable reference string (e.g. "%t0", "$1")
    /// Built per-function so names are sequential across the whole function.
    var_names: HashMap<usize, String>,
    /// All impure instruction indices across all labels of the current function.
    /// Used to skip cross-label dep lines (already printed in their own label).
    all_label_insts: HashSet<usize>,
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
            inline_set: HashSet::new(),
            var_names: HashMap::new(),
            all_label_insts: HashSet::new(),
        }
    }

    fn new_from_this(&self) -> Formatter<'a> {
        Formatter {
            inline_set: HashSet::new(),
            var_names: HashMap::new(),
            all_label_insts: HashSet::new(),
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
        let batch_view = self.ir.get_batch_view(labels_ptr);

        // ── Phase 1: build per-function variable name map ──
        let mut var_names: HashMap<usize, String> = HashMap::new();
        let mut all_label_insts: HashSet<usize> = HashSet::new();
        let mut counter = 0u32;

        for label_idx in 0..batch_view.values().len() {
            let label = batch_view.at(label_idx);
            let range = label.instruction_range();
            let impure_values = &self.ir.impure_instructions[range];

            // Collect all impure instruction indices (used to skip cross-label deps)
            for &v in impure_values {
                all_label_insts.insert(v.idx());
            }

            // Build inline set for this label (instructions that render inline)
            let inline_set = self.build_inline_set(label.value());
            let label_inst_set: HashSet<usize> =
                impure_values.iter().map(|v| v.idx()).collect();

            for &v in impure_values {
                let real_idx = v.idx();
                if inline_set.contains(&real_idx) {
                    continue;
                }
                if !var_names.contains_key(&real_idx) {
                    let instr = &self.instructions[real_idx];
                    if self.produces_value(instr) {
                        let prefix = if matches!(instr.opcode, Opcode::Allocate) {
                            "$"
                        } else {
                            "%t"
                        };
                        var_names.insert(real_idx, format!("{prefix}{counter}"));
                        counter += 1;
                    }
                }
            }

            // Assign names to unmapped deps too
            for &v in impure_values {
                let mut deps = BTreeSet::new();
                self.collect_unmapped_deps_with(
                    v.idx(),
                    &label_inst_set,
                    &inline_set,
                    &mut deps,
                );
                for dep_idx in deps {
                    if !var_names.contains_key(&dep_idx) {
                        let instr = &self.instructions[dep_idx];
                        if self.produces_value(instr) {
                            let prefix = if matches!(instr.opcode, Opcode::Allocate) {
                                "$"
                            } else {
                                "%t"
                            };
                            var_names.insert(dep_idx, format!("{prefix}{counter}"));
                            counter += 1;
                        }
                    }
                }
            }
        }

        let fmt = Formatter {
            var_names,
            all_label_insts,
            ..self.new_from_this()
        };

        // ── Phase 2: format labels using the map ──
        for label_idx in 0..batch_view.values().len() {
            out.push_str(&fmt.format_label(batch_view.at(label_idx)));
        }
        out.push_str("}\n");
        out
    }

    fn collect_unmapped_deps_with(
        &self,
        instr_idx: usize,
        label_inst_set: &HashSet<usize>,
        inline_set: &HashSet<usize>,
        out: &mut BTreeSet<usize>,
    ) {
        let instr = &self.instructions[instr_idx];
        for &op_val in &instr.operands {
            if op_val.is_void() {
                continue;
            }
            let dep_idx = op_val.idx();
            if matches!(
                self.instructions[dep_idx].opcode,
                Opcode::Const(_) | Opcode::RawValue | Opcode::Arg(_) | Opcode::BlockParam(_)
            ) {
                continue;
            }
            if inline_set.contains(&dep_idx) {
                continue;
            }
            if label_inst_set.contains(&dep_idx) {
                continue;
            }
            if out.insert(dep_idx) {
                self.collect_unmapped_deps_with(dep_idx, label_inst_set, inline_set, out);
            }
        }
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
            .collect::<Vec<_>>();

        let fields = params
            .iter()
            .enumerate()
            .map(|(idx, _)| format!("  %f{idx}: {} = p{idx};", params[idx]))
            .collect::<Vec<_>>();

        let children = comp_ty
            .children()
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let ty = self.ir.get_type(*c);
                let ty = if let IRType::Component(component) = ty {
                    self.fmt_component_type(&component)
                } else {
                    self.fmt_type(&ty)
                };

                format!("  #c{idx}: {ty};")
            })
            .collect::<Vec<_>>();
        let mut out = format!(
            "component %{}({}) {{\n",
            self.symbols.get_name(comp_ty.name()),
            params.join(","),
        );
        if !fields.is_empty() {
            out.push_str(&fields.join("\n"));
            out.push('\n');
        }
        if !children.is_empty() {
            out.push_str(&children.join("\n"));
            out.push('\n');
        }
        // UI instructions
        let ui_range = component.ui_instruction;
        let inline_set = self.build_component_inline_set(component);
        let fmt = Formatter {
            inline_set,
            var_names: self.var_names.clone(),
            all_label_insts: self.all_label_insts.clone(),
            ..*self
        };
        for i in 0..ui_range.len() {
            let idx = ui_range.ptr() + i;
            if fmt.inline_set.contains(&idx) {
                continue;
            }
            let inst = &self.instructions[idx];
            out.push_str("  ");
            out.push_str(&fmt.format_instruction(inst));
            out.push('\n');
        }

        out.push_str("}\n");
        out
    }

    // ── label formatting ──

    pub fn format_label(&self, label: IRViewer<'_, Label>) -> String {
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

        let inline_set = self.build_inline_set(label.value());
        let fmt = Formatter {
            inline_set,
            var_names: self.var_names.clone(),
            all_label_insts: self.all_label_insts.clone(),
            ..*self
        };

        let range = label.instruction_range();
        let impure_values = &self.ir.impure_instructions[range.clone()];

        let use_counts = self.build_use_counts(label.value());

        let label_inst_set: HashSet<usize> = impure_values.iter().map(|v| v.idx()).collect();

        let mut emitted = BTreeSet::new();
        let mut body = String::new();

        for value in impure_values.iter() {
            let real_idx = value.idx();
            let instr = &fmt.instructions[real_idx];

            if instr.opcode.is_inlineable() && use_counts.get(&real_idx).copied().unwrap_or(0) <= 1
            {
                continue;
            }

            let mut deps = BTreeSet::new();
            fmt.collect_unmapped_deps(real_idx, &mut deps);

            for dep_idx in deps {
                if label_inst_set.contains(&dep_idx) {
                    continue;
                }
                // Skip deps already defined in another label (printed there)
                if fmt.all_label_insts.contains(&dep_idx) {
                    continue;
                }
                if emitted.insert(dep_idx) {
                    let name = fmt.var_names.get(&dep_idx);
                    if let Some(name) = name {
                        body.push_str(&format!(
                            "  {name} = {}\n",
                            fmt.format_instruction(&fmt.instructions[dep_idx])
                        ));
                    } else {
                        body.push_str(&format!(
                            "  {} = {}\n",
                            fmt.format_instruction(&fmt.instructions[dep_idx]),
                            fmt.format_instruction(&fmt.instructions[dep_idx])
                        ));
                    }
                }
            }

            let line = fmt.format_instruction(instr);
            body.push_str("  ");
            if fmt.produces_value(instr) {
                let name = fmt.var_names.get(&real_idx).cloned().unwrap_or_default();
                body.push_str(&format!("{name} = {line}"));
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
                format!("p{idx}")
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
                if instr.operands.len() < 1 {
                    format!("@initcall {name}, {comp};")
                } else {
                    // The second operand is the style struct
                    format!("@initcall {name}, {comp};")
                }
            }
            Opcode::Struct | Opcode::Component => {
                let ty_str = self.fmt_type(&self.types.get_type(instr.value_type));
                let args = self.fmt_operands(&instr.operands);
                format!("{ty_str}{{{args}}}")
            }
            Opcode::GetChild(index) => format!("#c{index}"),
        }
    }

    // ── value formatting helpers ──

    fn fmt_value(&self, v: Value) -> String {
        if v.is_void() {
            return "void".to_string();
        }
        let idx = v.idx();
        let instr = &self.instructions[idx];

        // Always-inline ops: show literal value
        match &instr.opcode {
            Opcode::Arg(n) => return format!("p{n}"),
            Opcode::BlockParam(n) => return format!("lp{n}"),
            Opcode::Const(op) => return self.fmt_operand(op),
            Opcode::RawValue => {
                if !instr.operands.is_empty() {
                    return self.fmt_value(instr.operands[0]);
                }
            }
            _ => {}
        }

        // Inline-set ops (Struct/Component with ≤1 use): render inline
        if self.inline_set.contains(&idx) {
            return self.format_instruction(self.ir.get_instruction(v));
        }

        // If a variable name was assigned, use it
        if let Some(name) = self.var_names.get(&idx) {
            return name.clone();
        }

        // Fallback: render inline
        self.format_instruction(self.ir.get_instruction(v))
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
                Opcode::Const(_) | Opcode::RawValue | Opcode::Arg(_) | Opcode::BlockParam(_)
            ) {
                continue;
            }
            *counts.entry(dep).or_insert(0) += 1;
            if *counts.get(&dep).unwrap() == 1 {
                self.count_refs(dep, counts);
            }
        }
    }

    fn build_use_counts(&self, label: &Label) -> HashMap<usize, usize> {
        let range = label.instruction_range();
        let mut counts = HashMap::new();
        for &v in &self.ir.impure_instructions[range] {
            let idx = v.idx();
            let instr = &self.instructions[idx];
            for &op_val in &instr.operands {
                if !op_val.is_void() {
                    *counts.entry(op_val.idx()).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    fn build_inline_set(&self, label: &Label) -> HashSet<usize> {
        let range = label.instruction_range();
        let mut counts = HashMap::new();
        for &v in &self.ir.impure_instructions[range.clone()] {
            self.count_refs(v.idx(), &mut counts);
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

    fn build_component_inline_set(&self, component: &Component) -> HashSet<usize> {
        let ui_range = component.ui_instruction;
        let mut counts = HashMap::new();
        for i in 0..ui_range.len() {
            self.count_refs(ui_range.ptr() + i, &mut counts);
        }
        counts
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .map(|(idx, _)| idx)
            .collect()
    }

    fn collect_unmapped_deps(&self, instr_idx: usize, out: &mut BTreeSet<usize>) {
        let instr = &self.instructions[instr_idx];
        for &op_val in &instr.operands {
            let dep_idx = op_val.idx();
            if op_val.is_void()
                || self.instructions[dep_idx].opcode.is_inlineable()
                || self.inline_set.contains(&dep_idx)
            {
                continue;
            }

            if out.insert(dep_idx) {
                self.collect_unmapped_deps(dep_idx, out);
            }
        }
    }
}
