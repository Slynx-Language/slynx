use common::{ASTExpression, ASTExpressionKind, Span};
use frontend::hir::{
    SlynxHir,
    types::{FieldMethod, HirType},
};

fn generate_hir(kind: ASTExpressionKind) -> ASTExpression {
    let span = Span { start: 0, end: 0 };
    ASTExpression { kind, span }
}

#[test]
fn test_hir_tuple() {
    let tuple_ast = generate_hir(ASTExpressionKind::Tuple(vec![
        generate_hir(ASTExpressionKind::FloatLiteral(1.9)),
        generate_hir(ASTExpressionKind::IntLiteral(42)),
    ]));
    let mut hir = SlynxHir::new();
    let result = hir.resolve_expr(tuple_ast, None);
    assert!(result.is_ok(), "err in tuple hir: {:?}", result);
    let tuple_hir = result.unwrap();
    match tuple_hir.kind {
        frontend::hir::definitions::HirExpressionKind::Tuple(ref elements) => {
            assert_eq!(elements.len(), 2);
        }
        _ => panic!("expected HirExpressionKind::Tuple"),
    }
    match hir.types_module.get_type(&tuple_hir.ty) {
        HirType::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
        }
        _ => panic!("expected HirType::Tuple"),
    }
}

#[test]
fn test_hir_tuple_access_lowers_to_field_access() {
    let tuple_ast = generate_hir(ASTExpressionKind::TupleAccess {
        tuple: Box::new(generate_hir(ASTExpressionKind::Tuple(vec![
            generate_hir(ASTExpressionKind::IntLiteral(10)),
            generate_hir(ASTExpressionKind::StringLiteral("ok".to_string())),
        ]))),
        index: 1,
    });
    let mut hir = SlynxHir::new();
    let result = hir.resolve_expr(tuple_ast, None);
    assert!(result.is_ok(), "err in tuple access hir: {:?}", result);
    let tuple_access = result.unwrap();

    match tuple_access.kind {
        frontend::hir::definitions::HirExpressionKind::FieldAccess {
            ref expr,
            field_index,
        } => {
            assert_eq!(field_index, 1);
            assert!(matches!(
                expr.kind,
                frontend::hir::definitions::HirExpressionKind::Tuple(_)
            ));
        }
        _ => panic!("expected HirExpressionKind::FieldAccess"),
    }

    match hir.types_module.get_type(&tuple_access.ty) {
        HirType::Field(FieldMethod::Tuple(_, index)) => {
            assert_eq!(*index, 1);
        }
        _ => panic!("expected tuple access to keep tuple field metadata"),
    }
}
