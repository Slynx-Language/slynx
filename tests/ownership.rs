mod common;

#[test]
fn test_use_after_move_detected() {
    let source = r#"
object Box {
    value: int
}

func main(): void {
    let a = Box(value: 1);
    let b = a;
    let c = a;
}
"#;
    let err = common::compile_source(source).expect_err("should fail: use after move");
    assert!(err.contains("moved"), "Expected move error, got: {err}");
}

#[test]
fn test_move_through_assignment_detected() {
    let source = r#"
object Box {
    value: int
}

func main(): void {
    let a = Box(value: 1);
    let b = a;
    let c = a;
}
"#;
    let err = common::compile_source(source).expect_err("should fail: move through assignment");
    assert!(err.contains("moved"), "Expected move error, got: {err}");
}

#[test]
fn test_valid_move_independent_scope() {
    let source = r#"
object Box {
    value: int
}

func main(): void {
    let a = Box(value: 1);
    {
        let b = a;
    }
    let c = a;
}
"#;
    let result = common::compile_source(source);
    assert!(
        result.is_err(),
        "Expected ownership error for move in inner scope then use in outer scope"
    );
}

#[test]
fn test_move_then_borrow_detected() {
    let source = r#"
object Box {
    value: int
}

func main(): void {
    let a = Box(value: 1);
    let b = a;
    let r = &a;
}
"#;
    let err = common::compile_source(source).expect_err("should fail: borrow after move");
    assert!(err.contains("moved"), "Expected move error, got: {err}");
}

#[test]
fn test_valid_move_no_use_after() {
    let source = r#"
object Box {
    value: int
}

func main(): void {
    let a = Box(value: 1);
    let b = a;
}
"#;
    let result = common::compile_source(source);
    assert!(
        result.is_ok(),
        "Expected valid code to compile, got: {:?}",
        result.err()
    );
}

#[test]
fn test_multiple_function_args_are_moves() {
    let source = r#"
object Box {
    value: int
}

func take(b: Box): void {
}

func main(): void {
    let a = Box(value: 1);
    take(a);
    let b = a;
}
"#;
    let err = common::compile_source(source)
        .expect_err("should fail: use after move through function call");
    assert!(err.contains("moved"), "Expected move error, got: {err}");
}
