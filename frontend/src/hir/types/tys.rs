use crate::hir::{
    TypeId, VariableId,
    error::{HIRError, HIRErrorKind},
    symbols::SymbolPointer,
};
use common::ast::{GenericIdentifier, VisibilityModifier};

#[derive(Debug, Clone)]
///The method a field access is being made, whether it's a the field accessing to a type, or a variable
pub enum FieldMethod {
    ///Access the fields of a type directly. This can be understood by
    ///
    ///```rs
    /// object Person {name: str, age: int}
    /// func f(age:int): int{
    ///   let p = Person(name: "Maria", age:age);
    ///   p.age
    /// }```
    ///
    /// Since `p`'s type is Reference {rf: Person, generics: vec![]}, `p.age` is is Field(FieldMethod(Person, 1))
    Type(TypeId, usize),
    ///This is the same of the `type` variant, but since the provided `id` is the id of some variable whose type may be a Reference to a type, or
    ///a reference to another variable that references a type, we must store the field being accessed and check it on the type checker
    Variable(VariableId, SymbolPointer),
    ///Tuple accesses carry their numeric index from the parser so later phases
    /// can validate bounds without confusing them with named object fields.
    Tuple(TypeId, usize),
}

#[derive(Debug, Clone)]
pub enum HirType {
    Struct {
        fields: Vec<TypeId>,
    },

    Vector {
        ty: TypeId,
    },
    Tuple {
        fields: Vec<TypeId>,
    },
    ///This reference type can be understood better explained like
    ///object Name<T> {
    ///  value: T
    ///}
    ///func f(): Name<int> {
    ///  Name {value: 5}
    ///}
    ///Here, Name is the reference to the object type 'Name' with generic being 'int'
    Reference {
        ///The reference to the type this type maps to
        rf: TypeId,
        ///If its got a generic
        generics: Vec<TypeId>,
    },

    ///A type that references the type of another value. The provided `id` is the ID of this value
    VarReference(VariableId),

    ///The type of the Nth field on the struct/object with the provided `id`. If the struct is defined as
    /// struct S {a:int, b:str}, then Field(S_ID, 0) == int
    Field(FieldMethod),

    Function {
        args: Vec<TypeId>,
        return_type: TypeId,
    },
    ///A type used for booleans, which can be either true of false
    Bool,
    ///A type used for floats. This is by default the type of js.
    Float,
    ///A type used for ints. There's no Uint because js is gay. The difference between this to floats is that this is limited to be 32bits
    ///and it's optimized to use alot of byte operation to make things faster
    Int,

    ///Equivalent type of `string` in js
    Str,

    GenericComponent,
    ///A type specific for components
    Component {
        props: Vec<(VisibilityModifier, String, TypeId)>,
    },
    ///A type that represents no value
    Void,
    ///Type that must be resolved during type check
    Infer,
}

//On modificating some of the type ids, please check before on TypesModule, to see how the

impl HirType {
    ///Tries to retrieve a value from its `gener`(ic) type
    pub fn new(gener: &GenericIdentifier) -> Result<Self, HIRError> {
        match gener.identifier.as_str() {
            "Component" => Ok(Self::GenericComponent),
            "void" => Ok(Self::Void),
            "bool" => Ok(Self::Bool),
            "int" => Ok(Self::Int),
            "float" => Ok(Self::Float),
            "str" => Ok(Self::Str),
            _ => Err(HIRError {
                kind: HIRErrorKind::TypeNotRecognized(gener.to_string()),
                span: gener.span.clone(),
            }),
        }
    }
}
