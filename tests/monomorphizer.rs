use slynx_hir::SlynxHir;
use slynx_lexer::Lexer;
use slynx_monomorphizer::Monomorphizer;
use slynx_parser::Parser;

#[test]
fn rejects_cyclic_aliases() {
    let source = "alias A = B; alias B = A; func main(): void {}";
    let tokens = Lexer::tokenize(source).expect("source should tokenize");
    let declarations = Parser::new(tokens)
        .parse_declarations()
        .expect("source should parse");
    let mut hir = SlynxHir::new();
    let modules = vec![slynx_hir::module_loader::SourceNode::new(
        slynx_hir::module_loader::FileId::from_raw(0),
        declarations,
    )];
    hir.generate(&modules)
        .expect_err("HIR should reject cyclic types");

    Monomorphizer::resolve(&mut hir).expect("");
}
