use frontend::hir::{TypeId, types::{HirType, TypesModule}};

use crate::{IRType, SlynxIR, ir::temp::TempIRData};

impl SlynxIR {
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a struct type, using both `temp` and `tys` to resolve the fields. Panics if `decl` is not a struct type.
    pub(crate) fn insert_object_fields_for(&mut self, decl:TypeId, temp:&TempIRData, tys: &TypesModule) {
        
        let obj_handle = temp.get_type(decl);
        let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
            unreachable!();
        };

        let Some(HirType::Struct { fields }) = tys.get_object(&decl) else {
            unreachable!(
                "{:?} should map to an Object, but it doesn't",
                decl 
            );
        };
        for field in fields {
            let ty = self.get_ir_type(field, &temp, &tys);
            let obj_ty = self.types.get_object_type(obj);
            obj_ty.insert_field(ty);
        }
    }
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a function type, using both `temp` and `tys` to resolve the arguments and return type. Panics if `decl` is not a function type.
    pub(crate) fn insert_function_type_for(&mut self, decl:TypeId, temp:&TempIRData, tys: &TypesModule) {
        let HirType::Function { args, return_type } = tys.get_type(&decl)
        else {
            unreachable!();
        };
        let irty_id = temp.get_type(decl);
        let IRType::Function(func_tyid) = self.types.get_type(irty_id) else {
            unreachable!();
        };

        let mut extended_args = Vec::with_capacity(args.len());
        for arg in args {
            let arg_ty = self.get_ir_type(arg, &temp, &tys);
            extended_args.push(arg_ty);
        }
        let ret = self.get_ir_type(return_type, &temp, &tys);
        let ty = self.types.get_function_type(func_tyid);
        ty.insert_arg_types(&extended_args);
        ty.set_return_type(ret);
    }
    /// Inserts the contents of the provided `decl` into an IR struct type asserting it's a slynx component. This is made because component can be lowered to the equivalent of a struct with methods, thus 'classes'.
    /// The thing is that this is made interanlly with the minimum of abstraction as possible, so it becomes a struct and the components as well as methods are inserted directly into the struct as fields
    pub(crate) fn insert_component_fields_for(&mut self, decl:TypeId, temp:&TempIRData, tys: &TypesModule) {
        let obj_handle = temp.get_type(decl);
        let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
            unreachable!();
        };

        let Some(HirType::Component { props: ty_props }) =
            tys.get_object(&decl)
        else {
            unreachable!(
                "{:?} should map to an Object, but it doesn't",
               decl 
            );
        };

        for (_, _, prop) in ty_props {
            let ty = self.get_ir_type(prop, &temp, &tys);
            let obj_ty = self.types.get_object_type(obj);
            obj_ty.insert_field(ty);
        }
    }
}