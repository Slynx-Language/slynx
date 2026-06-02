#[test]
fn test_my_components() {
    let source = r#"
component Pedrinho {
    Div {
        Text {
            text: "Pedrinho leitazedo"
        }
    }
}

component Jorgin {
    Text {
        text: "Jorginho neguinho"
    }
    Pedrinho {}
}

func main(): Component {
    Jorgin {}
}
"#;
    let path = std::env::temp_dir().join("test_pedrinho.syx");
    std::fs::write(&path, source).unwrap();
    let ir = slynx::compile_to_ir(path).unwrap();
    println!("{}", ir.format_sir());
}
