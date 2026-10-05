//! Integration tests for the interfaces feature (see
//! `docs/contributing/interfaces.md`).
//!
//! This is an integration test *by contract*: it asserts which programs the
//! language must be able to compile once interfaces land. The feature is not
//! implemented yet, so every corpus file carries a `// xfail:` marker and is
//! asserted to (still) be rejected today. When an example starts compiling,
//! the harness reports it so the marker can be upgraded to a plain positive
//! example.

mod common;

/// Compiles every `.slx`/`.syx` example in `examples/interfaces/` and honors
/// the `// xfail:` / `// xpass:` markers, exactly like the `generics` and
/// `enums` corpora.
#[test]
fn interface_examples() {
    common::run_examples("examples/interfaces", "interface");
}

/// A method call on a bounded generic parameter resolves to the interface's
/// signature while the HIR is built and is discharged once the receiver's
/// concrete type is known.
#[test]
fn bounded_generic_dispatches_per_receiver() {
    common::compile_source_ok(
        r#"
interface Stringifiable {
    func stringify(&self) -> str
}

object Person {
    name: str
}

object Place {
    name: str
}

extend Person: Stringifiable {
    func stringify(&self): str -> self.name;
}

extend Place: Stringifiable {
    func stringify(&self): str -> self.name;
}

func describe<T>(x: T): str where T: Stringifiable -> x.stringify();

func main(): void {
    describe(Person(name: "ada"));
    describe(Place(name: "london"));
}
"#,
    );
}

/// The receiver is bound per specialization, so one generic body dispatches to
/// two different implementations.
#[test]
fn bounded_generic_selects_each_implementation() {
    let sir = common::compile_source_ok_sir(
        r#"
interface Stringifiable {
    func stringify(&self) -> str
}

object Person {
    name: str
}

object Place {
    name: str
}

extend Person: Stringifiable {
    func stringify(&self): str -> self.name;
}

extend Place: Stringifiable {
    func stringify(&self): str -> self.name;
}

func describe<T>(x: T): str where T: Stringifiable -> x.stringify();

func main(): void {
    describe(Person(name: "ada"));
    describe(Place(name: "london"));
}
"#,
    );
    assert!(
        sir.contains("describe_Person"),
        "the Person specialization should exist:\n{sir}"
    );
    assert!(
        sir.contains("describe_Place"),
        "the Place specialization should exist:\n{sir}"
    );
}

/// `Self` in a signature lowers to `Var(0)`, so it has to be substituted for
/// the receiver's concrete type when the call is discharged.
#[test]
fn bounded_generic_self_return_type() {
    common::compile_source_ok(
        r#"
interface Cloneable {
    func clone(&self) -> Self
}

object Counter {
    value: int
}

extend Counter: Cloneable {
    func clone(&self) -> Self {
        Counter(value: self.value)
    }
}

func duplicate<T>(x: T): T where T: Cloneable -> x.clone();

func main(): void {
    duplicate(Counter(value: 1));
}
"#,
    );
}

/// A `&mut self` receiver is dispatched through a bounded generic the same way
/// it is on a concrete type — the receiver only has to be mutable, and function
/// parameters never are.
#[test]
fn bounded_generic_mutable_receiver_needs_a_mutable_receiver() {
    common::compile_source_err_contains(
        r#"
interface Counter {
    func increment(&mut self): void
}

object Score {
    value: int
}

extend Score: Counter {
    func increment(&mut self): void {
        self.value = self.value + 1;
    }
}

func bump<T>(x: T): void where T: Counter -> x.increment();

func main(): void {
    bump(Score(value: 0));
}
"#,
        "immutable",
    );
}

/// A receiver whose type has no implementation of the interface method cannot
/// discharge the call.
#[test]
fn bounded_generic_without_implementation_is_rejected() {
    common::compile_source_err_contains(
        r#"
interface Stringifiable {
    func stringify(&self) -> str
}

object Person {
    name: str
}

func describe<T>(x: T): str where T: Stringifiable -> x.stringify();

func main(): void {
    describe(Person(name: "ada"));
}
"#,
        "no implementation",
    );
}

/// Without a `where` bound there is no interface to resolve the call against.
#[test]
fn unbounded_generic_call_is_rejected() {
    common::compile_source_err(
        r#"
interface Stringifiable {
    func stringify(&self) -> str
}

func describe<T>(x: T): str -> x.stringify();

func main(): void {
    describe(42);
}
"#,
    );
}
