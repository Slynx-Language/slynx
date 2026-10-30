mod common;

const ORDERED: &str = r#"
object Model {
    value: sint32,
    callback beforeSet(sint32): void before set(value) => (value);
    callback changed(sint32): void after set(value) => (value);
    callback read(sint32): void after get() => (return);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }

    func get(&self): sint32 -> self.value;
}

component View {
    prop seen: sint32 = 0;
    prop model: Model = Model(value: 0) {
        func beforeSet(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
        func changed(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
        func read(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
    }

    pub func run(&mut self): sint32 {
        self.model.set(7);
        self.model.get()
    }
}

func main(): sint32 {
    let mut view = View {};
    view.run()
}
"#;

#[test]
fn before_and_after_callbacks_wrap_the_method_in_order() {
    let sir = common::compile_source_ok_sir(ORDERED);
    let run = sir.split("i32 run(").nth(1).expect("run should be emitted");

    let before = run
        .find("__callback_View_model_beforeSet")
        .expect("before callback should be called");
    let method = run.find("= set(").expect("set should be called");
    let after = run
        .find("__callback_View_model_changed")
        .expect("after callback should be called");
    let get = run.find("= get(").expect("get should be called");
    let after_return = run
        .find("__callback_View_model_read")
        .expect("after-return callback should be called");

    assert!(
        before < method,
        "before callback must precede the method call"
    );
    assert!(method < after, "after callback must follow the method call");
    assert!(
        after < get,
        "callbacks must stay attached to their call site"
    );
    assert!(get < after_return, "return callback must follow get");
}

#[test]
fn callbacks_are_instrumented_in_every_expression_position() {
    let sir = common::compile_source_ok_sir(
        r#"
object Model {
    value: sint32,
    callback changed(sint32): void after bump(by) => (by);

    func bump(&mut self, by: sint32): sint32 {
        self.value = self.value + by;
        self.value
    }
}

component View {
    prop seen: sint32 = 0;
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
    }

    func passthrough(&mut self, value: sint32): sint32 -> value;

    pub func run(&mut self): sint32 {
        let first = self.model.bump(1);
        let second = self.passthrough(self.model.bump(2)) + self.model.bump(3);
        if self.model.bump(4) > 0 { first + second } else { 0 }
    }
}

func main(): sint32 {
    let mut view = View {};
    view.run()
}
"#,
    );
    assert_eq!(
        sir.matches("= __callback_View_model_changed").count(),
        4,
        "each call-site expression position must run the callback"
    );
}

#[test]
fn nested_child_property_callbacks_use_the_child_component() {
    let sir = common::compile_source_ok_sir(
        r#"
object Model {
    value: sint32,
    callback changed(sint32): void after bump(by) => (by);

    func bump(&mut self, by: sint32): sint32 {
        self.value = self.value + by;
        self.value
    }
}

component Child {
    pub prop seen: sint32 = 0;
    pub prop model: Model = Model(value: 0) {
        func changed(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
    }
}

component Parent {
    child=Child {}

    pub func run(&mut self): sint32 {
        self.child.model.bump(3)
    }
}

func main(): sint32 {
    let mut parent = Parent {};
    parent.run()
}
"#,
    );
    assert!(
        sir.contains("__callback_Child_model_changed"),
        "the callback implementation belongs to the child component"
    );
    let run = sir
        .split("i32 run(")
        .nth(1)
        .expect("parent run should be emitted");
    assert!(
        run.contains("__callback_Child_model_changed"),
        "a direct nested child property call must be instrumented"
    );
}

#[test]
fn property_callback_contract_must_be_declared_by_the_property_type() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop model: Model = Model(value: 0) {
        func missing(&mut self, value: sint32): void {}
    }
}

func main(): void {
    let mut view = View {};
}
"#,
        "is not declared by the property type",
    );
}

#[test]
fn return_cannot_be_mapped_before_the_method() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback invalid(sint32): void before get() => (return);

    func get(&self): sint32 -> self.value;
}

component View {
    prop model: Model = Model(value: 0) {
        func invalid(&mut self, value: sint32): void {}
    }
}

func main(): void {
    let mut view = View {};
}
"#,
        "cannot map the method return value",
    );
}

#[test]
fn calls_through_a_local_alias_are_not_instrumented() {
    let sir = common::compile_source_ok_sir(
        r#"
object Model {
    value: sint32,
    callback changed(sint32): void after bump(by) => (by);

    func bump(&mut self, by: sint32): sint32 {
        self.value = self.value + by;
        self.value
    }
}

component View {
    prop seen: sint32 = 0;
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: sint32): void {
            self.seen = self.seen + value;
        }
    }

    pub func run(&mut self): sint32 {
        let local = &mut self.model;
        local.bump(1)
    }
}

func main(): sint32 {
    let mut view = View {};
    view.run()
}
"#,
    );
    assert!(
        !sir.contains("= __callback_View_model_changed"),
        "only direct property receivers should be instrumented"
    );
}

#[test]
fn callback_contracts_must_return_void() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback invalid(sint32): sint32 after set(value) => (value);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop model: Model = Model(value: 0);
}

func main(): void {
    let view = View {};
}
"#,
        "must return 'void'",
    );
}

#[test]
fn callback_implementations_must_return_void() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback changed(sint32): void after set(value) => (value);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop seen: sint32 = 0;
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: sint32): sint32 {
            self.seen = value;
            value
        }
    }
}

func main(): sint32 {
    let mut view = View {};
    view.seen
}
"#,
        "must return 'void'",
    );
}

#[test]
fn callback_mapping_can_reorder_target_arguments() {
    let sir = common::compile_source_ok_sir(
        r#"
object Pair {
    first: sint32,
    second: sint32,
    callback swapped(sint32, sint32): void after set(first, second) => (second, first);

    func set(&mut self, first: sint32, second: sint32): void {
        self.first = first;
        self.second = second;
    }
}

component View {
    prop seen: sint32 = 0;
    prop pair: Pair = Pair(first: 0, second: 0) {
        func swapped(&mut self, second: sint32, first: sint32): void {
            self.seen = second - first;
        }
    }

    pub func run(&mut self): void {
        self.pair.set(1, 10);
    }
}

func main(): void {
    let mut view = View {};
    view.run();
}
"#,
    );
    let run = sir
        .split("void run(")
        .nth(1)
        .expect("run should be emitted");
    assert!(
        run.contains("__callback_View_pair_swapped_1(p0, 10, 1)"),
        "mapped arguments must be passed in contract order: {run}"
    );
}

#[test]
fn callback_implementation_parameter_types_must_match_the_contract() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback changed(sint32): void after set(value) => (value);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop seen: sint32 = 0;
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: bool): void {
            self.seen = self.seen + 1;
        }
    }
}

func main(): void {
    let view = View {};
}
"#,
        "Received an incorrect type",
    );
}

#[test]
fn callback_contract_type_count_must_match_the_mapping() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback changed(sint32, sint32): void after set(value) => (value);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: sint32): void {}
    }
}

func main(): void {
    let view = View {};
}
"#,
        "parameter type(s) but maps",
    );
}

#[test]
fn callback_contract_types_must_match_the_mapped_method_arguments() {
    common::compile_source_err_contains(
        r#"
object Model {
    value: sint32,
    callback changed(bool): void after set(value) => (value);

    func set(&mut self, value: sint32): void {
        self.value = value;
    }
}

component View {
    prop model: Model = Model(value: 0) {
        func changed(&mut self, value: bool): void {}
    }
}

func main(): void {
    let view = View {};
}
"#,
        "Received an incorrect type",
    );
}
