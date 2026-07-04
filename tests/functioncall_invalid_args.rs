mod common;

use crate::common::load_source;

#[test]
fn rejects_function_call_with_extra_arg() {
    let context =
        load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1, 2, 3) }");
    let err = context.compile().expect_err("Hir should fail its generation due to extra arg");
    let msg = format!("{err}");
    assert!(
        msg.contains("expected to receive 2 arguments"),
        "expected arg count mention in error, got: {msg}",
    );
    assert!(
        msg.contains("instead got 3 arguments"),
        "expected received arg count mention in error, got: {msg}",
    );
}

#[test]
fn rejects_function_call_with_missing_arg() {
    let context = load_source("func add(a: int, b: int): int { a + b } func main(): void { add(1) }");
    let err = context.compile().expect_err("Hir should fail its generation due to missing args");
    let msg = format!("{err}");
    assert!(
        msg.contains("expected to receive 2 arguments"),
        "expected arg count mention in error, got: {msg}",
    );
    assert!(
        msg.contains("instead got 1 arguments"),
        "expected received arg count mention in error, got: {msg}",
    );
}
