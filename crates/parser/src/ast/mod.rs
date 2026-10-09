mod component;
mod declarations;
mod expression;
mod imports;
mod statement;
mod types;

pub use common::VisibilityModifier;
use common::{FrontendSymbol, SymbolPointer};
pub use component::*;
pub use declarations::*;
pub use expression::*;
pub use imports::*;
pub use statement::*;
pub use types::*;

pub trait NamedASTDeclaration: ASTDeclaration {
    fn name(&self) -> SymbolPointer<FrontendSymbol>;
}

pub trait TypeASTDeclaration: ASTDeclaration {
    fn generics(&self) -> &GenericsMetadata;
}

pub trait ASTDeclaration {
    fn visibility(&self) -> VisibilityModifier;
}
