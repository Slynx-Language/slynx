mod common;

#[test]
fn test_components() {
    common::run_examples("examples/components", "components");
}

#[test]
fn component_literals_use_property_defaults() {
    let sir = common::compile_source_ok_sir(
        r#"
component Counter {
    prop value: sint32 = 42;
}

func main(): sint32 {
    let defaulted = Counter {};
    let explicit = Counter { value: 7 };
    0
}
"#,
    );

    assert!(sir.contains("Counter {%f0=42}"), "{sir}");
    assert!(sir.contains("Counter {%f0=7}"), "{sir}");
}

#[test]
fn component_literals_use_and_override_declared_child_defaults() {
    let sir = common::compile_source_ok_sir(
        r#"
component Text {
    prop value: sint32 = 1;
}

component Main {
    prop id: sint32 = 9;
    t=Text {}
}

func main(): sint32 {
    let defaulted = Main {};
    let overridden = Main { Text { value: 7 } };
    defaulted.id + defaulted.t.value + overridden.t.value
}
"#,
    );

    assert!(sir.contains("%Main {%f0=9, %c0=%Text {%f0=1}}"), "{sir}");
    assert!(sir.contains("%Main {%f0=9, %c0=%Text {%f0=7}}"), "{sir}");
}

#[test]
fn component_literals_reject_children_with_the_wrong_type() {
    let error = common::compile_source_err(
        r#"
component Text {}
component Other {}
component Main {
    t=Text {}
}

func main(): sint32 {
    let invalid = Main { Other {} };
    0
}
"#,
    );

    assert!(error.contains("Received an incorrect type"), "{error}");
}

#[test]
fn nested_component_defaults_are_lowered_as_child_values() {
    let sir = common::compile_source_ok_sir(
        r#"
object S {
    value: sint32,
    func empty(): Self {
        Self(value: 0)
    }
}

component Text {
    prop value: S = S.empty();
}

component Main {
    text=Text {}
}

func main(): sint32 {
    let main = Main {};
    main.text.value.value
}
"#,
    );

    assert!(sir.contains("%t2 = %Text {%f0=%t1}"), "{sir}");
    assert!(sir.contains("%t3 = %Main {%c0=%t2}"), "{sir}");
    assert!(sir.contains("S_empty()"), "{sir}");
}
