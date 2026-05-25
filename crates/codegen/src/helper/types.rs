use slynx_hir::{
    TypeId,
    model::{HirDeclaration, HirType},
};
use slynx_ir::{IRType, IRTypeId};

use crate::{Codegen, CodegenError};

impl Codegen {
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a struct type, using both `temp` and `tys` to resolve the fields. Panics if `decl` is not a struct type.
    pub(crate) fn insert_object_fields_for(&mut self, decl: TypeId) -> Result<(), CodegenError> {
        let obj_handle = self.temp.get_type(decl)?;
        let IRType::Struct(obj) = self.ir.get_type(obj_handle) else {
            unreachable!();
        };
        let Some(HirType::Struct { fields }) = self.temp.types_module().get_object(&decl) else {
            unreachable!("{:?} should map to an Object, but it doesn't", decl);
        };
        for field in fields {
            let ty = self.get_ir_type(&field)?;
            let obj_ty = self.types.get_object_type_mut(obj);
            obj_ty.insert_field(ty);
        }
        Ok(())
    }
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a function type, using both `temp` and `tys` to resolve the arguments and return type. Panics if `decl` is not a function type.
    pub(crate) fn insert_function_type_for(&mut self, decl: TypeId) -> Result<(), IRError> {
        let HirType::Function { args, return_type } = self.temp.types_module().get_type(&decl)
        else {
            unreachable!();
        };
        let irty_id = self.temp.get_type(decl)?;
        let IRType::Function(func_tyid) = self.types.get_type(irty_id) else {
            unreachable!();
        };

        let mut extended_args = Vec::with_capacity(args.len());
        for arg in args {
            let arg_ty = self.get_ir_type(arg)?;
            extended_args.push(arg_ty);
        }
        let ret = self.get_ir_type(return_type)?;
        let ty = self.types.get_function_type_mut(func_tyid);
        ty.insert_arg_types(&extended_args);
        ty.set_return_type(ret);
        Ok(())
    }
    ///Inserts the contents of the provided `decl` type, asserting it's the TypeId for a function type, using both `temp` and `tys` to resolve the arguments and return type. Panics if `decl` is not a function type.
    pub(crate) fn insert_stylesheet_type_for(
        &mut self,
        decl: &HirDeclaration,
    ) -> Result<(), IRError> {
        let HirType::Style { args } = self.types_module().get_type(&decl.ty) else {
            unreachable!();
        };

        let args_to_insert = args
            .iter()
            .map(|arg| self.get_ir_type(arg))
            .collect::<Result<Vec<_>, _>>()?;

        let f = self
            .temp
            .get_style_init_function(decl.id)
            .expect("Style should be hoisted");
        let ctx = self.get_context(f);
        if let IRType::Function(fty) = self.types.get_type(ctx.ty()) {
            let func_ty = self.types.get_function_type_mut(fty);
            func_ty.insert_arg_types(&args_to_insert);
        }

        Ok(())
    }
}
