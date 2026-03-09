mod context;
mod instruction;
mod label;
mod ptr;
mod temp;

pub use context::*;
pub use ptr::*;

use frontend::hir::{
    TypeId,
    definitions::{ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind},
    types::{HirType, TypesModule},
};

use crate::{
    IRType, IRTypeId, IRTypes,
    ir::{
        instruction::{Instruction, Operand},
        label::Label,
        temp::TempIRData,
    },
};

///All the IR containing contexts, labels, instructions and operands
pub struct SlynxIR {
    ///The contexts of this IR
    contexts: Vec<Context>,
    labels: Vec<Label>,
    instructions: Vec<Instruction>,
    operands: Vec<Operand>,
    types: IRTypes,
}

impl SlynxIR {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            labels: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            types: IRTypes::new(),
        }
    }

    ///Retrieves the context from its provided `ctx`
    pub(crate) fn get_context(&self, ctx: IRPointer<Context>) -> &Context {
        unsafe { &self.contexts[ctx.raw() as usize] }
    }

    ///Retrieves the context from its provided `ctx`
    pub(crate) fn get_context_mut(&mut self, ctx: IRPointer<Context>) -> &Context {
        unsafe { &mut self.contexts[ctx.raw() as usize] }
    }

    ///Creates a new blank function with no arguments and returning void. Returns its context handle
    fn create_blank_function(&mut self) -> IRPointer<Context> {
        let context = Context::new(self.types.create_empty_function());
        let ptr = self.contexts.len();
        self.contexts.push(context);
        IRPointer::new(ptr, 1)
    }

    ///Gets the type of the ir based on the provided `hirty`. Uses `temp` and `tymod` as auxiliary
    pub fn get_ir_type(&self, hirty: &TypeId, temp: &TempIRData, tymod: &TypesModule) -> IRTypeId {
        match *hirty {
            ty if ty == tymod.int_id() => self.types.int_type(),
            ty if ty == tymod.float_id() => self.types.float_type(),
            ty if ty == tymod.bool_id() => self.types.bool_type(),
            ty if ty == tymod.void_id() => self.types.void_type(),
            ty => temp.get_type(ty),
        }
    }

    pub fn generate(&mut self, hir: Vec<HirDeclaration>, tys: TypesModule) {
        let mut temp = TempIRData::new();
        //hoist of the objects
        for declaration in &hir {
            match &declaration.kind {
                frontend::hir::definitions::HirDeclarationKind::Object => {
                    let out = self.types.create_empty_struct();
                    temp.define_type(declaration.ty, out);
                    debug_assert!(out.0 == declaration.id.as_raw() as usize);
                }
                frontend::hir::definitions::HirDeclarationKind::Function { .. } => {
                    let out = self.create_blank_function();
                    let ctx = self.get_context(out);
                    temp.define_type(declaration.ty, ctx.ty());
                }
                HirDeclarationKind::ComponentDeclaration { .. } => {
                    let out = self.types.create_empty_struct();
                    temp.define_type(declaration.ty, out);
                    debug_assert!(out.0 == declaration.id.as_raw() as usize);
                }
            }
        }
        for declaration in hir {
            match declaration.kind {
                HirDeclarationKind::Object => {
                    let obj_handle = temp.get_type(declaration.ty);
                    let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
                        unreachable!();
                    };

                    let Some(HirType::Struct { fields }) = tys.get_object(&declaration.ty) else {
                        unreachable!(
                            "{:?} should map to an Object, but it doesn't",
                            declaration.ty
                        );
                    };
                    for field in fields {
                        let ty = self.get_ir_type(field, &temp, &tys);
                        let obj_ty = self.types.get_object_type(obj);
                        obj_ty.insert_field(ty);
                    }
                }
                HirDeclarationKind::Function { .. } => {
                    let HirType::Function { args, return_type } = tys.get_type(&declaration.ty)
                    else {
                        unreachable!();
                    };
                    let irty_id = temp.get_type(declaration.ty);
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
                HirDeclarationKind::ComponentDeclaration { props } => {
                    let obj_handle = temp.get_type(declaration.ty);
                    let IRType::Struct(obj) = self.types.get_type(obj_handle) else {
                        unreachable!();
                    };

                    let Some(HirType::Component { props: ty_props }) =
                        tys.get_object(&declaration.ty)
                    else {
                        unreachable!(
                            "{:?} should map to an Object, but it doesn't",
                            declaration.ty
                        );
                    };

                    for (_, _, prop) in ty_props {
                        let ty = self.get_ir_type(prop, &temp, &tys);
                        let obj_ty = self.types.get_object_type(obj);
                        obj_ty.insert_field(ty);
                    }
                    for prop in props {
                        match prop {
                            ComponentMemberDeclaration::Property { .. } => {}
                            ComponentMemberDeclaration::Specialized(_) => {}
                            ComponentMemberDeclaration::Child { .. } => {}
                        }
                    }
                }
            }
        }
    }
}
