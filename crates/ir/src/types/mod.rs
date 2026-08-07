mod components;
mod functions;
mod irtype;
mod structs;
mod tuple;

use common::pool::{DedupPool, DedupPoolId};
pub use components::*;
pub use functions::*;
pub use irtype::*;
pub use structs::*;
pub use tuple::*;

use crate::SymbolPointer;

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
    IRType::Specialized(IRSpecializedComponentType::Div),
    IRType::Specialized(IRSpecializedComponentType::Text),
];
pub type IRTypeId = DedupPoolId<IRType>;
#[derive(Debug)]
pub struct IRTypes {
    types: DedupPool<IRType>,
    structs: Vec<IRStruct>,
    functions: Vec<IRFunction>,
    components: Vec<IRComponent>,
}

impl std::default::Default for IRTypes {
    fn default() -> Self {
        Self::new()
    }
}

impl IRTypes {
    pub fn new() -> Self {
        let types = DedupPool::new();
        for builtin in BUILTIN_TYPES {
            types.insert(*builtin);
        }
        Self {
            types,
            structs: Vec::new(),
            functions: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn structs(&self) -> &[IRStruct] {
        &self.structs
    }
    pub fn components(&self) -> &[IRComponent] {
        &self.components
    }

    ///Checks if the provided `ty` is some variant of unsigned int
    pub fn is_negative_int(&self, ty: DedupPoolId<IRType>) -> bool {
        let typ = *self.types.get(ty);
        typ == IRType::U8
            || typ == IRType::U16
            || typ == IRType::U32
            || typ == IRType::U64
            || typ == IRType::USIZE
    }

    ///Retrieves the raw IR type from the provided `id`
    pub fn get_type(&self, id: IRTypeId) -> &IRType {
        self.types.get(id)
    }

    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_function_type(&self, id: IRFunctionId) -> &IRFunction {
        &self.functions[id.0]
    }

    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_object_type(&self, id: IRStructId) -> &IRStruct {
        &self.structs[id.0]
    }
    ///Gets a mutable referente to the type of the function with the provided `id`
    pub fn get_component_type(&self, id: IRComponentId) -> &IRComponent {
        &self.components[id.0]
    }

    ///Returns the IRTypeId of the `field_index`th field of the given struct/component type.
    ///Panics if `ty` is not a Struct or Component, or if `field_index` is out of bounds.
    pub fn get_field_type(&self, ty: IRTypeId, field_index: u16) -> IRTypeId {
        let field_index = field_index as usize;
        match self.types.get(ty) {
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

    #[inline]
    ///Inserts the given `ty` and returns its ID.
    pub fn insert_type(&self, ty: IRType) -> IRTypeId {
        self.types.insert(ty)
    }

    pub fn vector_type(&self, ty: IRTypeId) -> IRTypeId {
        let ty = IRType::Vector(ty);
        self.insert_type(ty)
    }

    ///Returns the int type
    pub fn int_type(&self) -> IRTypeId {
        self.insert_type(IRType::I32)
    }

    ///Returns the float type
    pub fn float_type(&self) -> IRTypeId {
        self.insert_type(IRType::F32)
    }

    ///Returns the bool type
    pub fn bool_type(&self) -> IRTypeId {
        self.insert_type(IRType::BOOL)
    }

    ///Returns the void type
    pub fn void_type(&self) -> IRTypeId {
        self.insert_type(IRType::VOID)
    }

    ///Returns the str type
    pub fn str_type(&self) -> IRTypeId {
        self.insert_type(IRType::STR)
    }

    ///Returns the usize type
    pub fn usize_type(&self) -> IRTypeId {
        self.insert_type(IRType::USIZE)
    }

    ///Returns the generic component type
    pub fn generic_component_type(&self) -> IRTypeId {
        self.insert_type(IRType::GenericComponent)
    }

    ///Returns the usize type
    pub fn specialized_div_type(&self) -> IRTypeId {
        self.insert_type(IRType::Specialized(IRSpecializedComponentType::Div))
    }

    ///Returns the generic component type
    pub fn specialized_text_type(&self) -> IRTypeId {
        self.insert_type(IRType::Specialized(IRSpecializedComponentType::Text))
    }
    ///Creates a new empty struct and returns its type ID
    pub(crate) fn create_empty_struct(&mut self, name: SymbolPointer) -> (IRTypeId, IRStructId) {
        let sout = self.structs.len();
        self.structs.push(IRStruct::new(Some(name)));
        let struct_id = IRStructId(sout);
        let out = self.insert_type(IRType::Struct(struct_id));
        (out, struct_id)
    }
    ///Creates a new empty struct and returns its type ID
    pub(crate) fn create_empty_component(
        &mut self,
        name: SymbolPointer,
    ) -> (IRTypeId, IRComponentId) {
        let sout = self.components.len();
        self.components.push(IRComponent::new(name));
        let component_id = IRComponentId(sout);
        let out = self.insert_type(IRType::Component(component_id));
        (out, component_id)
    }
    ///Creates a new empty function type with return `void`
    pub(crate) fn create_function_type(&mut self) -> (IRTypeId, IRFunctionId) {
        let fout = self.functions.len();
        self.functions.push(IRFunction::new(&[], self.void_type()));
        let func_id = IRFunctionId(fout);
        let out = self.insert_type(IRType::Function(func_id));
        (out, func_id)
    }
    pub fn create_or_get_tuple(&mut self, elements: Vec<IRTypeId>) -> IRTypeId {
        for (i, strukt) in self.structs.iter().enumerate() {
            if strukt.get_fields() == elements {
                return self.insert_type(IRType::Struct(IRStructId(i)));
            }
        }
        let mut s = IRStruct::new(None);
        for field in elements {
            s.insert_field(field);
        }
        let sid = IRStructId(self.structs.len());
        self.structs.push(s);
        self.insert_type(IRType::Struct(sid))
    }
}
