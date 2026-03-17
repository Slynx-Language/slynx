use frontend::hir::{
    TypeId,
    types::{HirType, TypesModule},
};

use crate::{
    IRError, IRType, IRTypeId, SlynxIR,
    ir::{
        model::{Context, IRPointer, Instruction, InstructionType, Operand, Value},
        temp::TempIRData,
    },
};

impl SlynxIR {
    #[inline]
    ///Gets the type of the ir based on the provided `hirty`. Uses `temp` and `tymod` as auxiliary
    pub fn get_ir_type(
        &self,
        hirty: &TypeId,
        temp: &TempIRData,
        tymod: &TypesModule,
    ) -> Result<IRTypeId, IRError> {
        Ok(match *hirty {
            ty if ty == tymod.int_id() => self.types.int_type(),
            ty if ty == tymod.float_id() => self.types.float_type(),
            ty if ty == tymod.bool_id() => self.types.bool_type(),
            ty if ty == tymod.void_id() => self.types.void_type(),
            ty if ty == tymod.generic_component_id() => self.types.generic_component_type(),
            ty => temp.get_type(ty)?,
        })
    }

    pub fn get_operand_type(&self, operand: IRPointer<Operand, 1>, _temp: &TempIRData) -> IRTypeId {
        match self.operands[operand.ptr()] {
            Operand::Bool(_) => self.types.bool_type(),
            Operand::Float(_) => self.types.float_type(),
            Operand::Int(_) => self.types.int_type(),
            Operand::String(_) => unimplemented!("Not implemented type for strings on operands"),
        }
    }

    pub fn get_type_of_value(&self, value: IRPointer<Value, 1>, temp: &TempIRData) -> IRTypeId {
        match &self.values[value.ptr()] {
            Value::FuncArg(idx) => self.arg_types_of_context(temp.current_function())[*idx],
            Value::Raw(operand) => self.get_operand_type(operand.clone(), temp),
            Value::Instruction(instr) => self.get_type_of_instruction(instr.clone(), temp),
            Value::LabelArg(_) => unimplemented!("Unimplemented type for label args"),
        }
    }

    pub fn get_type_of_instruction(
        &self,
        instr: IRPointer<Instruction, 1>,
        _temp: &TempIRData,
    ) -> IRTypeId {
        let instr = &self.instructions[instr.ptr()];
        instr.value_type
    }

    ///Creates a new blank function with no arguments and returning void. Returns its context handle
    pub fn create_blank_function(&mut self) -> IRPointer<Context> {
        let context = Context::new(self.types.create_empty_function());
        let ptr = self.contexts.len();
        self.contexts.push(context);
        IRPointer::new(ptr, 0)
    }

    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a struct type, using both `temp` and `tys` to resolve the fields. Panics if `decl` is not a struct type.
    pub(crate) fn insert_object_fields_for(
        &mut self,
        decl: TypeId,
        temp: &TempIRData,
        tys: &TypesModule,
    ) -> Result<(), IRError> {
        let obj_handle = temp.get_type(decl)?;
        let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
            unreachable!();
        };

        let Some(HirType::Struct { fields }) = tys.get_object(&decl) else {
            unreachable!("{:?} should map to an Object, but it doesn't", decl);
        };
        for field in fields {
            let ty = self.get_ir_type(field, &temp, &tys)?;
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
        tys: &TypesModule,
    ) -> Result<(), IRError> {
        let HirType::Function { args, return_type } = tys.get_type(&decl) else {
            unreachable!();
        };
        let irty_id = temp.get_type(decl)?;
        let IRType::Function(func_tyid) = self.types.get_type(irty_id) else {
            unreachable!();
        };

        let mut extended_args = Vec::with_capacity(args.len());
        for arg in args {
            let arg_ty = self.get_ir_type(arg, &temp, &tys)?;
            extended_args.push(arg_ty);
        }
        let ret = self.get_ir_type(return_type, &temp, &tys)?;
        let ty = self.types.get_function_type_mut(func_tyid);
        ty.insert_arg_types(&extended_args);
        ty.set_return_type(ret);
        Ok(())
    }
    /// Inserts the contents of the provided `decl` into an IR struct type asserting it's a slynx component. This is made because component can be lowered to the equivalent of a struct with methods, thus 'classes'.
    /// The thing is that this is made interanlly with the minimum of abstraction as possible, so it becomes a struct and the components as well as methods are inserted directly into the struct as fields
    pub(crate) fn insert_component_fields_for(
        &mut self,
        decl: TypeId,
        temp: &TempIRData,
        tys: &TypesModule,
    ) -> Result<(), IRError> {
        let obj_handle = temp.get_type(decl)?;
        let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
            unreachable!();
        };

        let Some(HirType::Component { props: ty_props }) = tys.get_component(&decl) else {
            unreachable!("{:?} should map to an Object, but it doesn't", decl);
        };

        for (_, _, prop) in ty_props {
            let ty = self.get_ir_type(prop, &temp, &tys)?;
            let obj_ty = self.types.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
}
