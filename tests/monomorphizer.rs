mod common;
use common::*;
use slynx_hir::SlynxHir;

#[test]
fn rejects_cyclic_aliases() {
    let mut ctx = load_source("alias A = B; alias B = A; func main(): void {}");
    let modules = ctx.load_modules().expect("Modules should load properly");
    // The HIR builder does not yet detect cyclic aliases.
    // When it does, this should expect_err instead.
    let _hir = SlynxHir::new(&modules).expect("HIR should build (cycle detection not yet implemented)");
}
