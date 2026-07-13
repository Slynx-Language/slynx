use common::Spanned;
use slynx_parser::ASTAttribute;

use crate::{HirAttribute, HirAttributeKind, SlynxHir, id::AnyDeclarationId};

/// Processes a list of AST attributes and returns their HIR representations.
///
/// Known attributes are handled immediately:
/// - `@builtin("name")` registers the declaration into `LangItems`.
/// - `@capabilities(...)` is stored for the effect system.
pub(crate) fn process_attributes(
    hir: &SlynxHir,
    attrs: &[Spanned<ASTAttribute>],
    decl_id: AnyDeclarationId,
) -> Vec<HirAttribute> {
    let mut out = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let kind = match hir.get_name(attr.data.name) {
            "builtin" => {
                let name = attr.data.args.first().copied().unwrap_or(attr.data.name);
                hir.lang_items.register(hir.get_name(name), decl_id);
                HirAttributeKind::Builtin { name }
            }
            "capabilities" => HirAttributeKind::Capabilities(attr.data.args.clone()),
            _ => HirAttributeKind::Unknown {
                name: attr.data.name,
                args: attr.data.args.clone(),
            },
        };
        out.push(HirAttribute {
            kind,
            span: attr.span,
        });
    }
    out
}
