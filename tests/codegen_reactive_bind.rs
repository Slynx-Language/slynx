mod common;

#[test]
fn codegen_emits_reactive_binds() {
    let sir = common::compile_source_ok_sir(
        r#"
component Child { in value: sint32; }
component Counter {
  pub prop count: sint32 = 0;
  view doubled = count * 2;
  c = Child { value: count }
  d = Child { value: doubled }
}
func main(): Counter { Counter { count: 5 } }
"#,
    );
    println!("{}", sir);
    assert!(
        sir.contains("@bind %f0 -> #c0, 0;"),
        "missing direct bind:\n{}",
        sir
    );
    assert!(
        sir.contains("@bind %f0 |> Counter_bind -> #c1, 0;"),
        "missing view bind:\n{}",
        sir
    );
}
