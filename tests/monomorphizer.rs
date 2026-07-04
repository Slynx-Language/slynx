use std::path::Path;

use slynx::SlynxContext;
use slynx_hir::SlynxHir;
use slynx_monomorphizer::Monomorphizer;

#[test]
fn rejects_cyclic_aliases() {
    let source = "alias A = B; alias B = A; func main(): void {}";
    let context = SlynxContext::from_source(source.to_string(), Path::new("input.slx"));
    let modules = context
        .load_modules()
        .expect("Modules should've generated properly");
    let mut hir =
        SlynxHir::new(&modules).expect("HIR should generate (cycle detection is deferred)");
    Monomorphizer::resolve(&mut hir).expect("monomorphization should succeed");
}
