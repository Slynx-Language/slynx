mod components;
mod functions;
mod irtype;
mod structs;
pub use components::*;
pub use functions::*;
pub use irtype::*;
pub use structs::*;

pub const BUILTIN_TYPES: &[IRType] = &[
    IRType::I8,
    IRType::U8,
    IRType::I16,
    IRType::U16,
    IRType::I32,
    IRType::U32,
    IRType::I64,
    IRType::U64,
    IRType::ISIZE,
    IRType::USIZE,
    IRType::F32,
    IRType::F64,
    IRType::STR,
    IRType::BOOL,
    IRType::VOID,
    IRType::GenericComponent,
];

#[derive(Debug, Default)]
pub struct IRTypes {
    types: Vec<IRType>,
    structs: Vec<IRStruct>,
    functions: Vec<IRFunction>,
    components: Vec<IRComponent>,
}

impl IRTypes {
    pub fn new() -> Self {
        Self {
            types: BUILTIN_TYPES.to_vec(),
            structs: Vec::new(),
            functions: Vec::new(),
            components: Vec::new(),
        }
    }

    ///Retrieves the raw IR type from the provided `id`
    pub fn get_type(&self, id: IRTypeId) -> IRType {
        self.types[id.0]
    }

    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_function_type(&self, id: IRFunctionId) -> &IRFunction {
        &self.functions[id.0]
    }

    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_object_type(&mut self, id: IRStructId) -> &IRStruct {
        &self.structs[id.0]
    }
    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_component_type(&self, id: IRComponentId) -> &IRComponent {
        &self.components[id.0]
    }

    ///Returns the IRTypeId of the `field_index`th field of the given struct/component type.
    ///Panics if `ty` is not a Struct or Component, or if `field_index` is out of bounds.
    pub fn get_field_type(&self, ty: IRTypeId, field_index: usize) -> IRTypeId {
        match self.types[ty.0] {
            IRType::Struct(sid) => self.structs[sid.0].get_fields()[field_index],
            IRType::Component(cid) => self.components[cid.0].fields[field_index],
            ref other => panic!(
                "Expected struct or component type for field access, got {:?}",
                other
            ),
        }
    }
    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_function_type_mut(&mut self, id: IRFunctionId) -> &mut IRFunction {
        &mut self.functions[id.0]
    }

    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_object_type_mut(&mut self, id: IRStructId) -> &mut IRStruct {
        &mut self.structs[id.0]
    }
    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_component_type_mut(&mut self, id: IRComponentId) -> &mut IRComponent {
        &mut self.components[id.0]
    }
    ///Returns the int type
    pub fn int_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::I32).unwrap())
    }

    ///Returns the float type
    pub fn float_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::F32).unwrap())
    }

    ///Returns the bool type
    pub fn bool_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::BOOL).unwrap())
    }

    ///Returns the void type
    pub fn void_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::VOID).unwrap())
    }

    ///Returns the str type
    pub fn str_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::STR).unwrap())
    }

    ///Returns the usize type
    pub fn usize_type(&self) -> IRTypeId {
        IRTypeId(self.types.iter().position(|v| *v == IRType::USIZE).unwrap())
    }

    ///Returns the generic component type
    pub fn generic_component_type(&self) -> IRTypeId {
        IRTypeId(
            self.types
                .iter()
                .position(|v| *v == IRType::GenericComponent)
                .unwrap(),
        )
    }

    ///Creates a new empty struct and returns its type ID
    pub fn create_empty_struct(&mut self) -> IRTypeId {
        let sout = self.structs.len();
        self.structs.push(IRStruct::new());
        let out = self.types.len();
        self.types.push(IRType::Struct(IRStructId(sout)));
        IRTypeId(out)
    }
    ///Creates a new empty struct and returns its type ID
    pub fn create_empty_component(&mut self) -> IRTypeId {
        let sout = self.structs.len();
        self.components.push(IRComponent::new());
        let out = self.types.len();
        self.types.push(IRType::Component(IRComponentId(sout)));
        IRTypeId(out)
    }
    ///Creates a new empty function type with return `void`
    pub fn create_empty_function(&mut self) -> IRTypeId {
        let fout = self.functions.len();
        self.functions.push(IRFunction::new(&[], self.void_type()));
        let out = self.types.len();
        self.types.push(IRType::Function(IRFunctionId(fout)));
        IRTypeId(out)
    }
}
