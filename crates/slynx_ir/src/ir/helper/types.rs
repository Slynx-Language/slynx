use common::SymbolPointer;
use slynx_hir::{TypeId, model::HirType};

use crate::{
    Component, IRComponent, IRError, IRType, IRTypeId, SlynxIR,
    ir::{
        model::{Context, IRPointer, Instruction, Operand},
        temp::TempIRData,
    },
};

impl SlynxIR {
    #[inline]
    ///Gets the type of the ir based on the provided `hirty`. Uses `temp` and `tymod` as auxiliary
    pub fn get_ir_type(&mut self, hirty: &TypeId, temp: &TempIRData) -> Result<IRTypeId, IRError> {
        let tymod = temp.types_module();
        Ok(match *hirty {
            ty if ty == tymod.int_id() => self.types.int_type(),
            ty if ty == tymod.float_id() => self.types.float_type(),
            ty if ty == tymod.bool_id() => self.types.bool_type(),
            ty if ty == tymod.void_id() => self.types.void_type(),
            ty if ty == tymod.str_id() => self.types.str_type(),
            ty if ty == tymod.generic_component_id() => self.types.generic_component_type(),
            ty => {
                if let Ok(mapped) = temp.get_type(ty) {
                    return Ok(mapped);
                }
                match tymod.get_type(&ty) {
                    HirType::Tuple { fields } => {
                        let fields = fields.clone();
                        let ir_fields = fields
                            .iter()
                            .map(|f| self.get_ir_type(f, temp))
                            .collect::<Result<Vec<_>, _>>()?;
                        self.types.create_or_get_tuple(ir_fields)
                    }
                    HirType::Reference { rf, .. } => {
                        let rf = *rf;
                        return self.get_ir_type(&rf, temp);
                    }
                    _ => {
                        return Err(IRError::IRTypeNotRecognized(ty));
                    }
                }
            }
        })
    }

    pub fn get_operand_type(&self, operand: IRPointer<Operand, 1>) -> IRTypeId {
        match self.operands[operand.ptr()] {
            Operand::Bool(_) => self.types.bool_type(),
            Operand::Float(_) => self.types.float_type(),
            Operand::Int(_) => self.types.int_type(),
            Operand::String(_) => self.types.str_type(),
        }
    }

    pub fn get_type_of_component(&self, component: IRPointer<Component, 1>) -> &IRComponent {
        let comp = self.get_component(component);
        let ty = self.types.get_type(comp.ty);
        let IRType::Component(c) = ty else {
            unreachable!("Type of component internally isnt a component? {ty:?}")
        };
        self.types.get_component_type(c)
    }

    pub fn get_type_of_instruction(&self, instr: IRPointer<Instruction, 1>) -> IRTypeId {
        let instr = &self.instructions[instr.ptr()];
        instr.value_type
    }

    ///Creates a new blank function with no arguments and returning void. Returns its context handle
    pub fn create_blank_function(&mut self, name: SymbolPointer) -> IRPointer<Context, 1> {
        let context = Context::new(name, self.types.create_empty_function());
        let ptr = self.contexts.len();
        self.contexts.push(context);
        IRPointer::new(ptr, 0)
    }
    ///Creates a new blank component with no arguments. Returns its context handle
    pub fn create_blank_component(&mut self, name: SymbolPointer) -> IRPointer<Component, 1> {
        let component = Component::new(
            self.types.create_empty_component(name),
            IRPointer::null(),
            IRPointer::null(),
        );
        let ptr = self.components.len();
        self.components.push(component);
        IRPointer::new(ptr, 0)
    }
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a struct type, using both `temp` and `tys` to resolve the fields. Panics if `decl` is not a struct type.
    pub(crate) fn insert_object_fields_for(
        &mut self,
        decl: TypeId,
        temp: &TempIRData,
    ) -> Result<(), IRError> {
        let obj_handle = temp.get_type(decl)?;
        let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
            unreachable!();
        };
        let Some(HirType::Struct { fields }) = temp.types_module().get_object(&decl) else {
            unreachable!("{:?} should map to an Object, but it doesn't", decl);
        };
        for field in fields {
            let ty = self.get_ir_type(field, temp)?;
            let obj_ty = self.types.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a function type, using both `temp` and `tys` to resolve the arguments and return type. Panics if `decl` is not a function type.
    pub(crate) fn insert_function_type_for(
        &mut self,
        decl: TypeId,
        temp: &TempIRData,
    ) -> Result<(), IRError> {
        let HirType::Function { args, return_type } = temp.types_module().get_type(&decl) else {
            unreachable!();
        };
        let irty_id = temp.get_type(decl)?;
        let IRType::Function(func_tyid) = self.types.get_type(irty_id) else {
            unreachable!();
        };

        let mut extended_args = Vec::with_capacity(args.len());
        for arg in args {
            let arg_ty = self.get_ir_type(arg, temp)?;
            extended_args.push(arg_ty);
        }
        let ret = self.get_ir_type(return_type, temp)?;
        let ty = self.types.get_function_type_mut(func_tyid);
        ty.insert_arg_types(&extended_args);
        ty.set_return_type(ret);
        Ok(())
    }
}
