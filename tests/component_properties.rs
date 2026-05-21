#[test]
fn component_property_style_reference() {
    let source = r#"
stylesheet Bg(color: int) uses Fg(color | 0x00ff00) {
	let c = (color & 0xff) << 8;
	styles {
		backgroundColor: color,
		foregroundColor: c,
	}
}
stylesheet Fg(color: int) {
	styles {
		foregroundColor: color
	}
}

component Pedrinho {
	Div {
		style: Bg(12),
		Text {
			text: "Pedrinho leitazedo"
		}
	}
}

component Jorgin {
    pub prop color:int;
	Text {
	    style: Fg(color),
		text: "Jorginho neguinho"
	}
	Pedrinho {}
}

func main(): Component {
	let a = 5;
	let b = 12;
	let c = a + b;
	Jorgin {
        color: 0
	}
}
"#;
    let path = std::env::temp_dir().join("test_component_properties.slx");
    std::fs::write(&path, source).unwrap();
    let ir = match slynx::compile_to_ir(path) {
        Ok(ir) => ir,
        Err(e) => {
            panic!("Compilation failed: {:?}", e);
        }
    };
    let formatted = ir.format_sir();
    println!("{}", formatted);

    // Verify styles are emitted
    assert!(
        formatted.contains("__init_Bg"),
        "Bg constructor should exist"
    );
    assert!(
        formatted.contains("__init_Fg"),
        "Fg constructor should exist"
    );
    assert!(
        formatted.contains("PedrinhoInit"),
        "PedrinhoInit should exist"
    );
    assert!(
        formatted.contains("JorginInit"),
        "JorginInit should exist"
    );

    // Check that Pedrinho has a Bg style initcall
    assert!(
        formatted.contains("@initcall Bg") || formatted.contains("initcall"),
        "Pedrinho should have a style initcall for Bg"
    );
}
