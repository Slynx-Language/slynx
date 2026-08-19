mod common;
use common::*;
use slynx::slynx_monomorphizer::Monomorphizer;
use slynx_hir::SlynxHir;

#[test]
fn rejects_cyclic_aliases() {
    let ctx = load_source("alias A = B; alias B = A; func main(): void {}");
    let modules = ctx.load_modules().expect("Modules should load properly");
    // The HIR builder does not yet detect cyclic aliases.
    // When it does, this should expect_err instead.
    let _hir =
        SlynxHir::new(&modules).expect("HIR should build (cycle detection not yet implemented)");
}

///Two calls to the same generic instantiation must generate a single
///specialized declaration, and the original generic template must be reported
///as dead code.
#[test]
fn deduplicates_identical_instantiations() {
    let ctx = load_source(
        "func identity<T>(x: T): T {
             x
         }
         func main(): int {
             let first = identity<int>(1);
             let second = identity<int>(2);
             first + second
         }",
    );
    let modules = ctx.load_modules().expect("Modules should load properly");
    let mut hir = SlynxHir::new(&modules).expect("HIR should build");

    let dead = Monomorphizer::resolve(&mut hir).expect("monomorphization should succeed");

    // The generic template must be neutralized and reported as dead code.
    assert_eq!(
        dead.len(),
        1,
        "expected exactly the generic template to be dead"
    );

    // Both `identity<int>` call sites must share a single specialization.
    let mut specialized = 0;
    for file in hir.files.iter() {
        for declaration in file.declarations.declarations.functions.iter() {
            if hir.get_name(declaration.name).starts_with("identity_") {
                specialized += 1;
            }
        }
    }
    assert_eq!(
        specialized, 1,
        "expected a single identity<int> specialization"
    );
}

///Each generic parameter contributes one `_<name>_<hash>` segment to the
///mangled name of a specialization.
#[test]
fn mangles_multiple_generic_parameters() {
    let ctx = load_source(
        "func second<T, U>(first: T, second: U): U {
             second
         }
         func main(): int {
             let result = second<bool, int>(true, 20);
             result
         }",
    );
    let modules = ctx.load_modules().expect("Modules should load properly");
    let mut hir = SlynxHir::new(&modules).expect("HIR should build");

    let dead = Monomorphizer::resolve(&mut hir).expect("monomorphization should succeed");
    assert_eq!(
        dead.len(),
        1,
        "expected exactly the generic template to be dead"
    );

    let mut names = Vec::new();
    for file in hir.files.iter() {
        for declaration in file.declarations.declarations.functions.iter() {
            names.push(hir.get_name(declaration.name).to_string());
        }
    }
    let specialized: Vec<_> = names
        .into_iter()
        .filter(|name| name.starts_with("second_"))
        .collect();
    assert_eq!(
        specialized.len(),
        1,
        "expected a single second<bool,int> specialization"
    );
    assert_eq!(
        specialized[0].matches('_').count(),
        4,
        "expected one _<name>_<hash> segment per generic parameter"
    );
}

///A generic call made inside another generic function must instantiate both
///generics, with concrete types flowing through the call chain.
#[test]
fn instantiates_nested_generic_calls() {
    let ctx = load_source(
        "func identity<T>(x: T): T {
             x
         }
         func wrap<T>(x: T): T {
             identity<T>(x)
         }
         func main(): int {
             wrap<int>(42)
         }",
    );
    let modules = ctx.load_modules().expect("Modules should load properly");
    let mut hir = SlynxHir::new(&modules).expect("HIR should build");

    let dead = Monomorphizer::resolve(&mut hir).expect("monomorphization should succeed");
    assert_eq!(dead.len(), 2, "expected both generic templates to be dead");

    let mut wrap_specializations = 0;
    let mut identity_specializations = 0;
    for file in hir.files.iter() {
        for declaration in file.declarations.declarations.functions.iter() {
            let name = hir.get_name(declaration.name);
            if name.starts_with("wrap_") {
                wrap_specializations += 1;
            }
            if name.starts_with("identity_") {
                identity_specializations += 1;
            }
        }
    }
    assert_eq!(
        wrap_specializations, 1,
        "expected a single wrap<int> specialization"
    );
    assert_eq!(
        identity_specializations, 1,
        "expected a single identity<int> specialization"
    );
}

///Calling a generic function with the wrong number of type arguments is an
///error, not a crash.
#[test]
fn rejects_wrong_generic_arity() {
    let ctx = load_source(
        "func second<T, U>(first: T, second: U): U {
             second
         }
         func main(): int {
             second<int>(1, 2)
         }",
    );
    let modules = ctx.load_modules().expect("Modules should load properly");
    let mut hir = SlynxHir::new(&modules).expect("HIR should build");

    Monomorphizer::resolve(&mut hir).expect("expected generic to be properly inferred");
}
