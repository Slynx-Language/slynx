use middleend::graph::{ReactiveBinding, ReactiveGraph, ReactiveGraphError};

fn binding(source: &str, target: &str, modifiers: &[&str]) -> ReactiveBinding {
    ReactiveBinding {
        source: source.into(),
        target: target.into(),
        modifiers: modifiers
            .iter()
            .map(|modifier| (*modifier).to_string())
            .collect(),
    }
}

#[test]
fn linearizes_simple_dependency_chain() {
    let mut graph = ReactiveGraph::new();
    graph.add_binding("%count", "%double_count", ["double"]);
    graph.add_binding("%double_count", "#text.0", std::iter::empty::<&str>());

    let bindings = graph.linearize().unwrap();

    assert_eq!(
        bindings,
        vec![
            binding("%count", "%double_count", &["double"]),
            binding("%double_count", "#text.0", &[]),
        ]
    );
}

#[test]
fn keeps_a_deterministic_order_for_parallel_dependencies() {
    let mut graph = ReactiveGraph::new();
    graph.add_binding("%count", "#label.0", std::iter::empty::<&str>());
    graph.add_binding("%count", "%double_count", ["double"]);
    graph.add_binding("%double_count", "#text.0", std::iter::empty::<&str>());
    graph.add_binding("%count", "#badge.0", std::iter::empty::<&str>());

    let bindings = graph.linearize().unwrap();

    assert_eq!(
        bindings,
        vec![
            binding("%count", "%double_count", &["double"]),
            binding("%count", "#badge.0", &[]),
            binding("%count", "#label.0", &[]),
            binding("%double_count", "#text.0", &[]),
        ]
    );
}

#[test]
fn ignores_duplicate_bindings() {
    let mut graph = ReactiveGraph::new();
    graph.add_binding("%count", "#text.0", ["fmt"]);
    graph.add_binding("%count", "#text.0", ["fmt"]);

    let bindings = graph.linearize().unwrap();

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.binding_count(), 1);
    assert_eq!(bindings, vec![binding("%count", "#text.0", &["fmt"])]);
}

#[test]
fn reports_cycles_with_the_remaining_nodes() {
    let mut graph = ReactiveGraph::new();
    graph.add_binding("%count", "%double_count", ["double"]);
    graph.add_binding("%double_count", "%view_state", std::iter::empty::<&str>());
    graph.add_binding("%view_state", "%count", std::iter::empty::<&str>());

    let error = graph.linearize().unwrap_err();

    assert_eq!(
        error,
        ReactiveGraphError::Cycle {
            nodes: vec![
                "%count".into(),
                "%double_count".into(),
                "%view_state".into(),
            ]
        }
    );
}
