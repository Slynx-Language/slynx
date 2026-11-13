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
    let mut vue = View {};
    vue.run()
}
"#;

#[test]
fn before_and_after_callbacks_wrap_the_method_in_order() {
    let sir = common::compile_source_ok_sir(ORDERED);
    let run = sir
        .split("i32 View_run(")
        .nth(1)
        .expect("run should be emitted");

    let before = run
        .find("View_View_callback1Ofmodel_beforeSet")
        .expect("before callback should be called");
    let method = run.find("Model_set(").expect("set should be called");
    let after = run
        .find("View_View_callback1Ofmodel_changed")
        .expect("after callback should be called");
    let get = run.find("Model_get(").expect("get should be called");
    let after_return = run
        .find("View_View_callback1Ofmodel_read")
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
    let mut vue = View {};
    vue.run()
}
"#,
    );
    assert_eq!(
        sir.matches("= View_View_callback1Ofmodel_changed").count(),
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
        sir.contains("Child_Child_callback1Ofmodel_changed"),
        "the callback implementation belongs to the child component"
    );
    let run = sir
        .split("i32 Parent_run(")
        .nth(1)
        .expect("parent run should be emitted");
    assert!(
        run.contains("Child_Child_callback1Ofmodel_changed"),
        "a direct nested child property call must be instrumented"
    );
}

#[test]
fn nested_component_method_runs_its_property_callback() {
    let sir = common::compile_source_ok_sir(
        r#"
extern {
    object Console {
        func log(&self, s: str): void;
    }
    pub static console: Console;
}

object S {
    s: str,
    callback onChange(str): void after update() => (content);

    func empty(): Self {
        Self(s: "")
    }

    func update(&mut self, content: str): void {
        self.s = content;
    }
}

component Text {
    pub prop v: S = S.empty() {
        func onChange(&mut self, newvalue: str): void {
            console.log(newvalue);
        }
    }

    pub func update(&mut self, n: str): void {
        self.v.update(n);
    }
}

component Main {
    prop value: sint32 = 0;
    t=Text {}

    func init(&mut self): void {
        self.t.update("Jorginho");
    }
}

func main(): Main {
    let mut m = Main {};
    m.init();
    return m;
}
"#,
    );
    let text_update = sir
        .split("void Text_update(")
        .nth(1)
        .expect("Text.update should be emitted");
    let update = text_update
        .find("S_update(")
        .expect("property method should be called");
    let callback = text_update
        .find("Text_Text_callback0Ofv_onChange(")
        .expect("property callback should be called");

    assert!(
        update < callback,
        "after callback must follow the property method call:\n{text_update}"
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
    let mut vue = View {};
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
    let mut vue = View {};
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
    let mut vue = View {};
    vue.run()
}
"#,
    );
    assert!(
        !sir.contains("= View_View_callback1Ofmodel_changed"),
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
    let vue = View {};
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
    let mut vue = View {};
    vue.seen
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
    let mut vue = View {};
    vue.run();
}
"#,
    );
    let run = sir
        .split("void View_run(")
        .nth(1)
        .expect("run should be emitted");
    assert!(
        run.contains("View_View_callback1Ofpair_swapped(p0, 10, 1)"),
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
    let vue = View {};
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
    let vue = View {};
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
    let vue = View {};
}
"#,
        "Received an incorrect type",
    );
}
