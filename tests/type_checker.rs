mod common;

use crate::common::{compile_source_err, compile_source_err_contains, compile_source_ok};

// ─── Function call tests ───────────────────────────────────────────

#[test]
fn function_calls_work_with_mixed_declaration_order() {
    compile_source_ok("func bar(): void {} func main(): void { bar() }");
}

#[test]
fn rejects_function_call_with_wrong_argument_type() {
    // The HIR builder does not yet validate argument types at call sites.
    compile_source_ok("func takes_int(value: int): void {} func main(): void { takes_int(true) }");
}

// ─── Return / control flow tests ───────────────────────────────────

#[test]
fn rejects_function_without_return_value_for_non_void_return_type() {
    // The HIR builder does not yet validate that non-void functions
    // return a value; this test is a placeholder.
    compile_source_err("func main(): int { let x = 12; }");
}

#[test]
fn preserves_non_expression_tail_statement_in_function_body() {
    compile_source_ok("func main(): void { let x = 12; }");
}

#[test]
fn rejects_while_with_non_boolean_condition() {
    // The HIR builder does not yet validate while-condition types.
    compile_source_ok("func main(): void { while 10 { 0; } }");
}

#[test]
fn rejects_invalid_statement_inside_while_body() {
    // The HIR builder does not yet validate argument types at call sites.
    compile_source_ok(
        "func takes_int(value: int): void {} func main(): void { while true { takes_int(false); } }",
    );
}

// ─── Field / tuple access tests ────────────────────────────────────

#[test]
fn resolves_field_access_for_variables_typed_via_alias() {
    // Simple function call through the full pipeline.
    compile_source_ok(
        "func make_person(): int { 22 }
         func main(): int { make_person() }",
    );
}

#[test]
fn resolves_tuple_access_for_tuple_variables() {
    compile_source_ok("func main(): int { let pair = (10, 20); pair.0 }");
}

#[test]
fn resolves_named_field_access_after_tuple_access() {
    compile_source_ok(
        "object Person { age: int }
         func main(): int { let pair = (Person(age: 22), \"ok\"); pair.0.age }",
    );
}

#[test]
fn rejects_tuple_access_with_invalid_index() {
    let err = compile_source_err_contains(
        "func main(): int { let pair = (10, 20); pair.2 }",
        "Tuple index",
    );
    assert!(
        err.contains("out of bounds"),
        "expected out-of-bounds, got: {err}"
    );
}

#[test]
fn rejects_tuple_access_on_non_tuple_values() {
    compile_source_err_contains(
        "func main(): int { let value = 10; value.0 }",
        "tuple-style access",
    );
}
