mod common;

#[test]
fn arithmetic_and_bitwise_compound_assignment_compiles() {
    common::compile_source_ok(
        "func main(): void {
            let mut x: sint32 = 1;
            x += 2;
            x -= 2;
            x *= 2;
            x /= 2;
            x %= 2;
            x <<= 1;
            x >>= 1;
            x &= 1;
            x |= 1;
            x ^= 1;
        }",
    );
}

#[test]
fn logical_compound_assignment_compiles() {
    common::compile_source_ok(
        "func main(): void {
            let mut x: bool = true;
            x &&= false;
            x ||= false;
        }",
    );
}
