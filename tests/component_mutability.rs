mod common;

use crate::common::{
    compile_source_ok, compile_source_err_contains,
};

#[test]
fn prop_can_be_mutated_by_owner() {
    compile_source_ok(
        "component C { pub prop value: sint32 = 0; pub func inc(&mut self): void { self.value += 1; } } func main(): C { let mut c = C {}; c.inc(); c }",
    );
}

#[test]
fn rejects_assignment_to_input() {
    compile_source_err_contains(
        "component C { in value: sint32; pub func bad(&mut self): void { self.value = 1; } } func main(): C { let mut c = C { value: 0 }; c.bad(); c }",
        "Expression is not assignable",
    );
}

#[test]
fn rejects_assignment_to_view() {
    compile_source_err_contains(
        "component C { pub prop a: sint32 = 0; view b = a * 2; pub func bad(&mut self): void { self.b = 1; } } func main(): C { let mut c = C {}; c.bad(); c }",
        "Expression is not assignable",
    );
}

#[test]
fn rejects_mutable_borrow_of_input() {
    compile_source_err_contains(
        "component C { in value: sint32; pub func bad(&mut self): void { let r = &mut self.value; } } func main(): C { let mut c = C { value: 0 }; c.bad(); c }",
        "Expression is not assignable",
    );
}

#[test]
fn rejects_compound_assignment_to_input() {
    compile_source_err_contains(
        "component C { in value: sint32; pub func bad(&mut self): void { self.value += 1; } } func main(): C { let mut c = C { value: 0 }; c.bad(); c }",
        "Expression is not assignable",
    );
}