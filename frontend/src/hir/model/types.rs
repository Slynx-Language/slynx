use common::{SymbolPointer, VisibilityModifier};

use crate::hir::{TypeId, VariableId};

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

///A type to represent the property of some component
#[derive(Debug, Clone)]
pub struct ComponentProperty(VisibilityModifier, String, TypeId);

impl ComponentProperty {
    pub fn new(visibility: VisibilityModifier, name: String, ty: TypeId) -> Self {
        Self(visibility, name, ty)
    }

    pub fn new_public(name: String, ty: TypeId) -> Self {
        Self::new(VisibilityModifier::Public, name, ty)
    }

    pub fn new_private(name: String, ty: TypeId) -> Self {
        Self::new(VisibilityModifier::Private, name, ty)
    }

    pub fn visibility(&self) -> &VisibilityModifier {
        &self.0
    }
    pub fn name(&self) -> &str {
        &self.1
    }
    pub fn prop_type(&self) -> &TypeId {
        &self.2
    }
}

#[derive(Debug, Clone)]
///A type that will exist on the HIR. This is used to determine the types of things so the IR can generate specific code for each of them
pub enum HirType {
    Struct {
        fields: Vec<TypeId>,
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
        props: Vec<ComponentProperty>,
    },
    ///A type that represents no value
    Void,
    ///Type that must be resolved during type check
    Infer,
}

//On modificating some of the type ids, please check before on TypesModule, to see how the

impl HirType {
    ///Tries to retrieve a value from its `gener`(ic) type name
    pub fn new(generic: &str) -> Option<Self> {
        match generic {
            "Component" => Some(Self::GenericComponent),
            "void" => Some(Self::Void),
            "bool" => Some(Self::Bool),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "str" => Some(Self::Str),
            _ => None,
        }
    }

    ///Creates a new Struct type with the provided `fields`
    pub fn new_struct(fields: Vec<TypeId>) -> Self {
        Self::Struct { fields }
    }
    ///Creates a new Tuple type with the provided `fields`. The pattern is [int,float], is the same as (int,float)
    pub fn new_tuple(fields: Vec<TypeId>) -> Self {
        Self::Tuple { fields }
    }

    ///Creates a new generic reference to the given `rf` with the given `generics`
    pub fn new_generic_ref(rf: TypeId, generics: Vec<TypeId>) -> Self {
        Self::Reference { rf, generics }
    }

    ///Creates a new generic reference to the given `rf` without generics
    pub fn new_ref(rf: TypeId) -> Self {
        Self::new_generic_ref(rf, Vec::new())
    }

    ///Creates a new function type with the given `args` and `return_type`
    pub fn new_function(args: Vec<TypeId>, return_type: TypeId) -> Self {
        Self::Function { args, return_type }
    }
    ///Creates a new function type with the given `args` and `return_type`
    pub fn new_component(props: Vec<ComponentProperty>) -> Self {
        Self::Component { props }
    }
}
