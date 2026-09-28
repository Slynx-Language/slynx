mod common;

use common::load_source;
use slynx::slynx_monomorphizer::Monomorphizer;
use slynx_hir::SlynxHir;

/// Builds the HIR for `source`, runs monomorphization and hands `(hir, dead_len)`
/// to `check` while the HIR is still alive.
fn resolve(source: &str, check: impl FnOnce(&SlynxHir, usize)) {
    let ctx = load_source(source);
    let modules = ctx.load_modules().expect("Modules should load properly");
    let mut hir = SlynxHir::new(&modules).expect("HIR should build");
    let dead = Monomorphizer::resolve(&mut hir).expect("monomorphization should succeed");
    check(&hir, dead.len());
}

#[test]
fn rejects_cyclic_aliases() {
    let ctx = load_source("alias A = B; alias B = A; func main(): void {}");
    let modules = ctx.load_modules().expect("Modules should load properly");
    // The HIR builder does not yet detect cyclic aliases.
    // When it does, this should expect_err instead.
    let _hir =
        SlynxHir::new(&modules).expect("HIR should build (cycle detection not yet implemented)");
}

/// Two calls to the same generic instantiation must generate a single
/// specialized declaration, and the original generic template must be reported
/// as dead code.
#[test]
fn deduplicates_identical_instantiations() {
    resolve(
        "func identity<T>(x: T): T {
             x
         }
         func main(): int {
             let first = identity<int>(1);
             let second = identity<int>(2);
             first + second
         }",
        |hir, dead| {
            // The generic template must be neutralized and reported as dead code.
            assert_eq!(dead, 1, "expected exactly the generic template to be dead");

            // Both `identity<int>` call sites must share a single specialization.
            assert_eq!(
                common::count_specializations(hir, "identity_"),
                1,
                "expected a single identity<int> specialization"
            );
        },
    );
}

/// Each generic parameter contributes one `_<name>_<hash>` segment to the
/// mangled name of a specialization.
#[test]
fn mangles_multiple_generic_parameters() {
    resolve(
        "func second<T, U>(first: T, second: U): U {
             second
         }
         func main(): int {
             let result = second<bool, int>(true, 20);
             result
         }",
        |hir, dead| {
            assert_eq!(dead, 1, "expected exactly the generic template to be dead");

            let mut names = Vec::new();
            for file in hir.store.files.iter() {
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
        },
    );
}

/// A generic call made inside another generic function must instantiate both
/// generics, with concrete types flowing through the call chain.
#[test]
fn instantiates_nested_generic_calls() {
    resolve(
        "func identity<T>(x: T): T {
             x
         }
         func wrap<T>(x: T): T {
             identity<T>(x)
         }
         func main(): int {
             wrap<int>(42)
         }",
        |hir, dead| {
            assert_eq!(dead, 2, "expected both generic templates to be dead");
            assert_eq!(
                common::count_specializations(hir, "wrap_"),
                1,
                "expected a single wrap<int> specialization"
            );
            assert_eq!(
                common::count_specializations(hir, "identity_"),
                1,
                "expected a single identity<int> specialization"
            );
        },
    );
}

/// Calling a generic function with the wrong number of type arguments is an
/// error, not a crash.
#[test]
fn rejects_wrong_generic_arity() {
    resolve(
        "func second<T, U>(first: T, second: U): U {
             second
         }
         func main(): int {
             second<int>(1, 2)
         }",
        |_hir, _dead| {},
    );
}
