use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReactiveNodeId(usize);

impl ReactiveNodeId {
    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReactiveNodeKey(String);

impl ReactiveNodeKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ReactiveNodeKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ReactiveNodeKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactiveBinding {
    pub source: ReactiveNodeKey,
    pub target: ReactiveNodeKey,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactiveGraphError {
    Cycle { nodes: Vec<ReactiveNodeKey> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReactiveEdge {
    target: ReactiveNodeId,
    modifiers: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ReactiveGraph {
    nodes: Vec<ReactiveNodeKey>,
    node_mapping: HashMap<ReactiveNodeKey, ReactiveNodeId>,
    outgoing: Vec<Vec<ReactiveEdge>>,
    bindings: usize,
}

impl ReactiveGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn binding_count(&self) -> usize {
        self.bindings
    }

    pub fn ensure_node<K>(&mut self, key: K) -> ReactiveNodeId
    where
        K: Into<ReactiveNodeKey>,
    {
        let key = key.into();
        if let Some(id) = self.node_mapping.get(&key).copied() {
            return id;
        }

        let id = ReactiveNodeId(self.nodes.len());
        self.nodes.push(key.clone());
        self.node_mapping.insert(key, id);
        self.outgoing.push(Vec::new());
        id
    }

    pub fn add_binding<S, T, M, I>(&mut self, source: S, target: T, modifiers: I)
    where
        S: Into<ReactiveNodeKey>,
        T: Into<ReactiveNodeKey>,
        M: Into<String>,
        I: IntoIterator<Item = M>,
    {
        let source = self.ensure_node(source);
        let target = self.ensure_node(target);
        let modifiers = modifiers.into_iter().map(Into::into).collect::<Vec<_>>();

        let edges = &mut self.outgoing[source.as_usize()];
        if edges
            .iter()
            .any(|edge| edge.target == target && edge.modifiers == modifiers)
        {
            return;
        }

        edges.push(ReactiveEdge { target, modifiers });
        self.bindings += 1;
    }

    pub fn linearize(&self) -> Result<Vec<ReactiveBinding>, ReactiveGraphError> {
        let mut indegree = vec![0usize; self.nodes.len()];
        for edges in &self.outgoing {
            for edge in edges {
                indegree[edge.target.as_usize()] += 1;
            }
        }

        let mut ready = BTreeSet::new();
        for (index, key) in self.nodes.iter().enumerate() {
            if indegree[index] == 0 {
                ready.insert((key.clone(), ReactiveNodeId(index)));
            }
        }

        let mut ordered_nodes = Vec::with_capacity(self.nodes.len());
        while let Some((key, node)) = ready.pop_first() {
            let _ = key;
            ordered_nodes.push(node);

            let mut edges = self.outgoing[node.as_usize()].clone();
            self.sort_edges(&mut edges);

            for edge in edges {
                let target = edge.target.as_usize();
                indegree[target] -= 1;
                if indegree[target] == 0 {
                    ready.insert((self.nodes[target].clone(), ReactiveNodeId(target)));
                }
            }
        }

        if ordered_nodes.len() != self.nodes.len() {
            let mut nodes = indegree
                .iter()
                .enumerate()
                .filter(|(_, degree)| **degree > 0)
                .map(|(index, _)| self.nodes[index].clone())
                .collect::<Vec<_>>();
            nodes.sort();
            return Err(ReactiveGraphError::Cycle { nodes });
        }

        let mut bindings = Vec::with_capacity(self.bindings);
        for node in ordered_nodes {
            let mut edges = self.outgoing[node.as_usize()].clone();
            self.sort_edges(&mut edges);

            for edge in edges {
                bindings.push(ReactiveBinding {
                    source: self.nodes[node.as_usize()].clone(),
                    target: self.nodes[edge.target.as_usize()].clone(),
                    modifiers: edge.modifiers,
                });
            }
        }

        Ok(bindings)
    }

    fn sort_edges(&self, edges: &mut [ReactiveEdge]) {
        edges.sort_by(|lhs, rhs| {
            self.edge_rank(lhs.target)
                .cmp(&self.edge_rank(rhs.target))
                .then_with(|| {
                    self.nodes[lhs.target.as_usize()].cmp(&self.nodes[rhs.target.as_usize()])
                })
                .then_with(|| lhs.modifiers.cmp(&rhs.modifiers))
        });
    }

    fn edge_rank(&self, node: ReactiveNodeId) -> usize {
        usize::from(self.outgoing[node.as_usize()].is_empty())
    }
}
