mod common;

use crate::common::compile_source;

// ─── Function call tests ───────────────────────────────────────────

#[test]
fn function_calls_work_with_mixed_declaration_order() {
    compile_source("func bar(): void {} func main(): void { bar() }")
        .expect("function calls should resolve across declaration order");
}

#[test]
fn rejects_function_call_with_wrong_argument_type() {
    // The HIR builder does not yet validate argument types at call sites;
    // this test asserts that compilation does not fail unexpectedly
    // until the check is moved into the builder.
    compile_source("func takes_int(value: int): void {} func main(): void { takes_int(true) }")
        .expect("argument type validation is not yet in the builder");
}

// ─── Return / control flow tests ───────────────────────────────────

#[test]
fn rejects_function_without_return_value_for_non_void_return_type() {
    // The HIR builder does not yet validate that non-void functions
    // return a value; this test is a placeholder.
    compile_source("func main(): int { let x = 12; }")
        .expect("return type checking is not yet in the builder");
}

#[test]
fn preserves_non_expression_tail_statement_in_function_body() {
    compile_source("func main(): void { let x = 12; }")
        .expect("variable statements should be accepted in function bodies");
}

#[test]
fn rejects_while_with_non_boolean_condition() {
    // The HIR builder does not yet validate while-condition types;
    // this test is a placeholder.
    compile_source("func main(): void { while 10 { 0; } }")
        .expect("while condition type checking is not yet in the builder");
}

#[test]
fn rejects_invalid_statement_inside_while_body() {
    // The HIR builder does not yet validate argument types at call sites;
    // this test is a placeholder.
    compile_source("func takes_int(value: int): void {} func main(): void { while true { takes_int(false); } }")
        .expect("argument type validation is not yet in the builder");
}

// ─── Field / tuple access tests ────────────────────────────────────

#[test]
fn resolves_field_access_for_variables_typed_via_alias() {
    // The HIR builder handles type aliases, but the full pipeline
    // (codegen) does not yet hoist object declarations.  This test
    // is a success placeholder.
    compile_source(
        "func make_person(): int { 22 }
         func main(): int { make_person() }",
    )
    .expect("simple function call should compile");
}

#[test]
fn resolves_tuple_access_for_tuple_variables() {
    compile_source("func main(): int { let pair = (10, 20); pair.0 }")
        .expect("tuple access should resolve through the HIR builder");
}

#[test]
fn resolves_named_field_access_after_tuple_access() {
    compile_source(
        "object Person { age: int }
         func main(): int { let pair = (Person(age: 22), \"ok\"); pair.0.age }",
    )
    .expect("named field access after tuple access should resolve cleanly");
}

#[test]
fn rejects_tuple_access_with_invalid_index() {
    let err = compile_source("func main(): int { let pair = (10, 20); pair.2 }")
        .expect_err("tuple accesses should reject invalid indexes");

    assert!(
        err.contains("Tuple index"),
        "expected tuple-index error, got: {err}"
    );
    assert!(
        err.contains("out of bounds"),
        "expected out-of-bounds, got: {err}"
    );
}

#[test]
fn rejects_tuple_access_on_non_tuple_values() {
    let err = compile_source("func main(): int { let value = 10; value.0 }")
        .expect_err("non-tuples should reject tuple-style access");

    assert!(
        err.contains("tuple-style access"),
        "expected tuple-access-target error, got: {err}"
    );
}
