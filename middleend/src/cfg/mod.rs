use std::collections::{HashMap, VecDeque};

use petgraph::{algo::Cycle, graph::NodeIndex, prelude::StableDiGraph};

use crate::{IRPointer, InstructionType, Label, SlynxIR};

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
    pub fn new(label_ptr: IRPointer<Label, 1>, ir: &SlynxIR) -> Self {
        let mut label_to_node = HashMap::new();
        let mut graph = StableDiGraph::new();
        let mut queue = VecDeque::new();
        queue.push_back(label_ptr);
        while let Some(current) = queue.pop_front() {
            if label_to_node.contains_key(&current) {
                continue;
            }
            let label = ir.get_label(current);
            if let Some((successors, kinds)) = Self::get_sucessors_of(label, ir) {
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

    pub fn get_sucessors_of(
        label: &Label,
        ir: &SlynxIR,
    ) -> Option<(Vec<IRPointer<Label, 1>>, Vec<EdgeKind>)> {
        let instructions = ir.get_label_instructions(label).into_iter().flatten();
        let mut sucessors = Vec::new();
        let kind = match instructions.last()?.instruction_type {
            InstructionType::Br(c) => {
                sucessors.push(c);
                vec![EdgeKind::Unconditional]
            }
            InstructionType::Cbr {
                then_label,
                else_label,
                ..
            } => {
                sucessors.push(then_label);
                sucessors.push(else_label);
                vec![EdgeKind::ConditionalTrue, EdgeKind::ConditionalFalse]
            }
            _ => vec![EdgeKind::Backedge],
        };
        Some((sucessors, kind))
    }

    pub fn graph(&self) -> &CfgGraph {
        &self.graph
    }
    pub fn entry(&self) -> IRPointer<Label, 1> {
        self.entry
    }
    ///Returns a hashmap that maps a label(which is in fact used) to its index on the graph
    pub fn label_mappings(&self) -> &HashMap<IRPointer<Label, 1>, NodeIndex> {
        &self.label_to_node
    }

    pub fn topological_order(&self) -> Result<Vec<NodeIndex>, Cycle<NodeIndex>> {
        petgraph::algo::toposort(&self.graph, None)
    }
}
