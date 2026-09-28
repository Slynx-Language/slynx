mod common;

use crate::common::{compile_source_err, compile_source_err_contains, compile_source_ok};

const BOX_SOURCE: &str = r#"
object Box {
    value: int
}
"#;

#[test]
fn test_use_after_move_detected() {
    let source = format!(
        "{BOX_SOURCE}
func main(): void {{
    let a = Box(value: 1);
    let b = a;
    let c = a;
}}
"
    );
    compile_source_err_contains(&source, "moved");
}

#[test]
fn test_valid_move_independent_scope() {
    let source = format!(
        "{BOX_SOURCE}
func main(): void {{
    let a = Box(value: 1);
    {{
        let b = a;
    }}
    let c = a;
}}
"
    );
    let _err = compile_source_err(&source);
}

#[test]
fn test_move_then_borrow_detected() {
    let source = format!(
        "{BOX_SOURCE}
func main(): void {{
    let a = Box(value: 1);
    let b = a;
    let r = &a;
}}
"
    );
    compile_source_err_contains(&source, "moved");
}

#[test]
fn test_valid_move_no_use_after() {
    let source = format!(
        "{BOX_SOURCE}
func main(): void {{
    let a = Box(value: 1);
    let b = a;
}}
"
    );
    compile_source_ok(&source);
}

#[test]
fn test_multiple_function_args_are_moves() {
    let source = format!(
        "{BOX_SOURCE}
func take(b: Box): void {{
}}

func main(): void {{
    let a = Box(value: 1);
    take(a);
    let b = a;
}}
"
    );
    compile_source_err_contains(&source, "moved");
}
