use crate::{
    middleend::hir::{
        TypeId,
<<<<<<< HEAD
        types::{FieldMethod, HirType, TypesModule},
=======
        types::{HirType, TypesModule},
>>>>>>> 673350bdf83de9e71f0201d3f72330e2818494c1
    },
    middleend::intermediate::IntermediateRepr,
};

#[derive(Debug, Clone)]
pub enum IntermediateType {
    Void,
    Int,
    Float,
    Bool,
    Component,
    Str,
    ///Index inside the intermediate repr
    Complex(Vec<IntermediateType>),
    Vector(Box<IntermediateType>), //vec of a type
    Reference(TypeId),
    ///A function. The first value is the args, and the second one is the return type
    Function(Vec<IntermediateType>, Box<IntermediateType>),
}

impl IntermediateRepr {
    ////Creates a new complex type from the provided `tys` and returns it
<<<<<<< HEAD
    pub fn retrieve_complex(&self, tys: &[TypeId], module: &TypesModule) -> IntermediateType {
=======
    pub fn retrieve_complex(&mut self, tys: &[TypeId], module: &TypesModule) -> IntermediateType {
>>>>>>> 673350bdf83de9e71f0201d3f72330e2818494c1
        IntermediateType::Complex(tys.iter().map(|t| self.get_type(t, module)).collect())
    }

    pub fn get_type(&self, ty: &TypeId, module: &TypesModule) -> IntermediateType {
        match module.get_type(ty) {
            HirType::Int => IntermediateType::Int,
            HirType::Float => IntermediateType::Float,
            HirType::Bool => IntermediateType::Bool,
            HirType::Str => IntermediateType::Str,
            HirType::Void => IntermediateType::Void,
            HirType::Vector { ty } => {
                let vecty = self.get_type(ty, module);
                IntermediateType::Vector(Box::new(vecty))
            }
<<<<<<< HEAD
            HirType::Struct { fields } => self.retrieve_complex(fields, module),
            HirType::Component { .. } | HirType::GenericComponent => IntermediateType::Component,
            HirType::Function { args, return_type } => IntermediateType::Function(
                args.iter().map(|arg| self.get_type(arg, module)).collect(),
                Box::new(self.get_type(return_type, module)),
            ),
            HirType::Reference { rf, .. } => IntermediateType::Reference(*rf),
            HirType::VarReference(var_id) => {
                let Some(var_ty) = module.get_variable(var_id) else {
                    unreachable!("Variable type should be known before lowering");
                };
                self.get_type(var_ty, module)
            }
            HirType::Field(FieldMethod::Type(parent, field_idx)) => {
                let struct_ty = match module.get_type(parent) {
                    HirType::Reference { rf, .. } => module.get_type(rf),
                    other => other,
                };
                let HirType::Struct { fields } = struct_ty else {
                    unreachable!("Field parent should resolve to a struct type");
                };
                let Some(field_ty) = fields.get(*field_idx) else {
                    unreachable!(
                        "Field index should be in bounds during lowering. parent={parent:?}, field_idx={field_idx}"
                    );
                };
                self.get_type(field_ty, module)
            }
            HirType::Field(FieldMethod::Variable(var_id, name)) => {
                unreachable!(
                    "Field-by-variable type should be resolved during type checking. var={var_id:?}, field={name:?}"
                )
            }
            HirType::Infer => {
                unreachable!("Infer type should be fully resolved before IR lowering")
=======
            HirType::GenericComponent => IntermediateType::Component,
            HirType::Reference { rf, .. } => IntermediateType::Reference(*rf),
            v @ (HirType::Function { .. } | HirType::Struct { .. } | HirType::Component { .. }) => {
                unreachable!("All struct types should be reference instead. Got {v:?}");
            }
            HirType::VarReference(_) | HirType::Field(_) | HirType::Infer => {
                unreachable!("These should have been resolved during checker")
>>>>>>> 673350bdf83de9e71f0201d3f72330e2818494c1
            }
        }
    }
}
<<<<<<< HEAD

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleend::hir::{VariableId, types::HirType};

    #[test]
    fn resolves_var_reference_to_concrete_type() {
        let mut types = TypesModule::new();
        let var = VariableId::new();
        types.insert_variable(var, types.int_id());
        let var_ref = types.insert_unnamed_type(HirType::VarReference(var));

        let ir = IntermediateRepr::new();
        let lowered = ir.get_type(&var_ref, &types);
        assert!(matches!(lowered, IntermediateType::Int));
    }

    #[test]
    fn resolves_field_type_to_concrete_field_type() {
        let mut types = TypesModule::new();
        let struct_ty = types.insert_unnamed_type(HirType::Struct {
            fields: vec![types.bool_id()],
        });
        let struct_ref = types.insert_unnamed_type(HirType::Reference {
            rf: struct_ty,
            generics: Vec::new(),
        });
        let field_ty = types.insert_unnamed_type(HirType::Field(FieldMethod::Type(struct_ref, 0)));

        let ir = IntermediateRepr::new();
        let lowered = ir.get_type(&field_ty, &types);
        assert!(matches!(lowered, IntermediateType::Bool));
    }
}
=======
>>>>>>> 673350bdf83de9e71f0201d3f72330e2818494c1
