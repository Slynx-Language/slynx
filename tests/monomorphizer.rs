use slynx_hir::{SlynxHir, error::HIRErrorKind};
use slynx_lexer::Lexer;
use slynx_monomorphizer::Monomorphizer;
use slynx_parser::Parser;
use slynx_typechecker::TypeChecker;

#[test]
fn rejects_cyclic_aliases_in_monomorphization() {
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
    hir.generate(&modules).expect("HIR should generate");

    let mut hir = TypeChecker::check(hir).expect("type checking should pass");

    let err = Monomorphizer::resolve(&mut hir)
        .expect_err("cyclic aliases should fail during monomorphization");

    match &err.kind {
        HIRErrorKind::RecursiveType { ty } => assert_eq!(*ty, hir.intern_name("A")),
        other => panic!("expected RecursiveType, got {other:?}"),
    }
}
