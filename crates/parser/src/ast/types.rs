use std::hash::Hash;

use common::{
    FrontendSymbol, Spanned, SymbolsModule, VisibilityModifier,
    pool::{DedupPool, DedupPoolId},
};
use smallvec::SmallVec;

use crate::{ASTExpression, SymbolPointer};

#[derive(Debug)]
///A name that is typed. This is simply the representation of `name: kind`
pub struct TypedName {
    pub name: Spanned<SymbolPointer>,
    pub kind: Spanned<DedupPoolId<Type>>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Type {
    Plain(GenericIdentifier),
    Array(DedupPoolId<Type>, DedupPoolId<ASTExpression>),
    Vector(DedupPoolId<Type>),
    Nullable(DedupPoolId<Type>),
    ///A reference to the i-th type parameter of the enclosing generic declaration.
    ///For example, in `func A<T>(arg: T): T`, both the `arg` type and the return
    ///type are represented as `Generic(0)`.
    Generic(u8),
    Reference(DedupPoolId<Type>),
    MutableReference(DedupPoolId<Type>),
}

///Renders a [`Type`] back to a human-readable name using the shared interning
///pools. `generic_names` maps a [`Type::Generic`] index back to the name of the
///corresponding type parameter of the enclosing generic declaration.
///
///The `types`, `symbols` and `expressions` pools are shared between the parser
///and the module loader, so this single function is used by both instead of two
///duplicated implementations. Nullable array/vector types are parenthesized
///(`([]int)?`) to disambiguate them from `([]int?)`.
pub fn type_name(
    types: &DedupPool<Type>,
    symbols: &SymbolsModule<FrontendSymbol>,
    expressions: &DedupPool<ASTExpression>,
    ty: DedupPoolId<Type>,
    generic_names: &[SymbolPointer],
) -> SymbolPointer {
    match &types[ty] {
        Type::Reference(inner) => {
            let name =
                symbols.get_name(type_name(types, symbols, expressions, *inner, generic_names));
            symbols.intern(&format!("&{}", name))
        }
        Type::MutableReference(inner) => {
            let name =
                symbols.get_name(type_name(types, symbols, expressions, *inner, generic_names));
            symbols.intern(&format!("&mut {}", name))
        }
        Type::Plain(gi) => gi.identifier,
        Type::Array(arr, len) => {
            let name =
                symbols.get_name(type_name(types, symbols, expressions, *arr, generic_names));
            let len = match expressions.get(*len) {
                ASTExpression::IntLiteral(int) => int.to_string(),
                _ => unimplemented!(
                    "This is not supported. An array type should contain a number inside it to determine its size. This is an expression due to the possibility of comptime, that is idealized. But at the moment only integer literals are accepted"
                ),
            };
            symbols.intern(&format!("[{len}]{name}"))
        }
        Type::Vector(inner) => symbols.intern(&format!(
            "[]{}",
            symbols.get_name(type_name(types, symbols, expressions, *inner, generic_names))
        )),
        Type::Nullable(inner) => {
            let inner_name =
                symbols.get_name(type_name(types, symbols, expressions, *inner, generic_names));
            match types.get(*inner) {
                Type::Array(_, _) | Type::Vector(_) => {
                    symbols.intern(&format!("({inner_name})?"))
                }
                _ => symbols.intern(&format!("{inner_name}?")),
            }
        }
        Type::Generic(index) => generic_names[*index as usize],
    }
}

///A context to determine what the type is being related to. This can contain information of generic names, at the moment
pub struct TypeContext<'a> {
    pub generic_names: &'a [SymbolPointer],
}

impl<'a> TypeContext<'a> {
    pub const fn new(generics: &'a [SymbolPointer]) -> Self {
        Self {
            generic_names: generics,
        }
    }
}
impl TypeContext<'static> {
    pub const EMPTY: Self = TypeContext::new(&[]);
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
///A Identifier that might contain a generic. Such as `Component<int>`
pub struct GenericIdentifier {
    ///The generic this identifier contains.
    pub generic: SmallVec<[Spanned<DedupPoolId<Type>>; 2]>,
    ///The name of this identifier
    pub identifier: SymbolPointer,
}
#[derive(Debug)]
pub struct ObjectField {
    pub visibility: VisibilityModifier,
    pub name: Spanned<TypedName>,
}
