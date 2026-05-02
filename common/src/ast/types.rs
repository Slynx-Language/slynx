use crate::ast::Span;

#[derive(Debug)]
///A name that is typed. This is simply the representation of `name: kind`
pub struct TypedName {
    pub name: String,
    pub kind: GenericIdentifier,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
///A Identifier that might contain a generic. Such as `Component<int>`
pub struct GenericIdentifier {
    ///The generic this identifier contains.
    pub generic: Option<Vec<GenericIdentifier>>,
    ///The name of this identifier
    pub identifier: String,
    pub span: Span,
}

impl std::fmt::Display for GenericIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref generic) = self.generic {
            write!(
                f,
                "{}<{}>",
                self.identifier,
                generic
                    .iter()
                    .map(|g| g.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        } else {
            write!(f, "{}", self.identifier)
        }
    }
}
