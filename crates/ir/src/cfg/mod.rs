use std::collections::{HashMap, VecDeque};

use petgraph::{algo::Cycle, graph::NodeIndex, prelude::StableDiGraph};

use crate::{IRPointer, IRStorage, Label, Opcode, SlynxIR};

type CfgGraph = StableDiGraph<BasicBlock, EdgeKind>;

#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    Unconditional,
    ConditionalTrue,
    ConditionalFalse,
    Backedge,
}

pub struct BasicBlock {
    label: IRPointer<Label, 1>,
    successors: Vec<IRPointer<Label, 1>>,
}

impl BasicBlock {
    pub fn new(label: IRPointer<Label, 1>, successors: Vec<IRPointer<Label, 1>>) -> Self {
        Self { label, successors }
    }

    pub fn label(&self) -> IRPointer<Label, 1> {
        self.label
    }

    pub fn successors(&self) -> &[IRPointer<Label, 1>] {
        &self.successors
    }
}

pub struct ControlFlowGraph {
    graph: CfgGraph,
    entry: IRPointer<Label, 1>,
    label_to_node: HashMap<IRPointer<Label, 1>, NodeIndex<u32>>,
}

impl ControlFlowGraph {
    /// Build a CFG from the instruction stream owned by `label_ptr`'s function.
    ///
    /// `label_ptr` is a pointer into `ir.labels` with dynamic length;
    /// labels within that range are treated as the function's basic
    /// blocks.  Successors are computed by inspecting the **last**
    /// instruction of each block.
    pub fn new(label_ptr: IRPointer<Label, 1>, ir: &SlynxIR) -> Self {
        let mut label_to_node = HashMap::new();
        let mut graph = StableDiGraph::new();
        let mut queue = VecDeque::new();
        queue.push_back(label_ptr);
        while let Some(current) = queue.pop_front() {
            if label_to_node.contains_key(&current) {
                continue;
            }
            let label = ir.get_idx(current.ptr());
            if let Some((successors, kinds)) = Self::get_successors_of(label, ir) {
                let index = graph.add_node(BasicBlock::new(current, successors.clone()));
                label_to_node.insert(current, (index, kinds));
                queue.extend(successors);
            }
        }
        let mut final_stack = Vec::new();
        for (index, kinds) in label_to_node.values() {
            if let Some(block) = graph.node_weight(*index) {
                for ((sindex, _), kind) in block
                    .successors
                    .iter()
                    .filter_map(|s| label_to_node.get(s))
                    .zip(kinds)
                {
                    final_stack.push((*index, *sindex, *kind));
                }
            }
        }
        while let Some((index, target, kind)) = final_stack.pop() {
            graph.add_edge(index, target, kind);
        }
        let mut out = HashMap::with_capacity(label_to_node.len());
        for (label, (index, _)) in label_to_node {
            out.insert(label, index);
        }
        Self {
            graph,
            label_to_node: out,
            entry: label_ptr,
        }
    }

    /// Extract successors by looking at the terminator (last instruction)
    /// of the given block.
    fn get_successors_of(
        label: &Label,
        ir: &SlynxIR,
    ) -> Option<(Vec<IRPointer<Label, 1>>, Vec<EdgeKind>)> {
        let range = label.instruction_range();
        let last = ir.instructions[range].last()?;

        match &last.opcode {
            Opcode::Br(target) => {
                Some((vec![*target], vec![EdgeKind::Unconditional]))
            }
            Opcode::Cbr {
                then_label,
                else_label,
            } => {
                Some((
                    vec![*then_label, *else_label],
                    vec![EdgeKind::ConditionalTrue, EdgeKind::ConditionalFalse],
                ))
            }
            _ => {
                // No terminator or non-branching end → back-edge (treated as exit)
                Some((vec![], vec![EdgeKind::Backedge]))
            }
        }
    }

    pub fn graph(&self) -> &CfgGraph {
        &self.graph
    }
    pub fn entry(&self) -> IRPointer<Label, 1> {
        self.entry
    }
    pub fn label_mappings(&self) -> &HashMap<IRPointer<Label, 1>, NodeIndex> {
        &self.label_to_node
    }

    pub fn topological_order(&self) -> Result<Vec<NodeIndex>, Cycle<NodeIndex>> {
        petgraph::algo::toposort(&self.graph, None)
    }
}
