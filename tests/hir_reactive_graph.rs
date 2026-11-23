mod common;

use common::load_source;

#[test]
fn reactive_graph_success() {
    let ctx = load_source(
        r#"
component Child { in value: sint32; }
component Counter {
  pub prop count: sint32 = 0;
  view doubled = count * 2;
  c = Child { value: doubled }
}
func main(): Counter { Counter {} }
"#,
    );
    let modules = ctx.load_modules().unwrap();
    let (hir, _dead, _ownership) = ctx.build_hir(&modules).unwrap();

    let mut checked = false;
    for file in hir.store.files.iter() {
        for (id, decl) in file.declarations.components.iter().with_ids() {
            let name = hir.get_name(decl.name);
            if name != "Counter" {
                continue;
            }
            let cid = slynx_hir::DeclarationId::new(file.file, id);
            let graph = hir.component_graphs.get(&cid).expect("graph should exist");
            // count, doubled, and one child field
            assert_eq!(graph.graph.node_count(), 3);
            // count -> doubled, doubled -> child.value
            assert_eq!(graph.graph.edge_count(), 2);
            checked = true;
        }
    }
    assert!(checked, "Counter component graph was not checked");
}

#[test]
fn reactive_graph_rejects_cyclic_view() {
    let ctx = load_source(
        r#"
component C {
  pub prop a: sint32 = 0;
  view x = y + 1;
  view y = x + 1;
}
func main(): C { C {} }
"#,
    );
    let modules = ctx.load_modules().unwrap();
    let result = ctx.build_hir(&modules);
    assert!(result.is_err(), "cyclic reactive dependency should be rejected");
}
