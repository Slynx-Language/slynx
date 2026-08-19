use std::ops::Deref;

use common::pool::DedupPoolId;

use crate::{
    ComponentType, FunctionType, HirType, StructType, StyleType, TupleType,
    helpers::views::HirViewer,
};

impl HirViewer<'_, DedupPoolId<HirType>> {
    pub fn raw(&self) -> &HirType {
        &self.hir.types_module[self.data]
    }

    pub fn name(&self) -> String {
        match &self.hir.types_module[self.dereference().data] {
            HirType::Bool => "bool".to_string(),
            HirType::Float => "float32".to_string(),
            HirType::Int => "int".to_string(),
            HirType::Void => "void".to_string(),
            HirType::GenericComponent => "anycomponent".to_string(),
            HirType::Nullable(ty) => {
                let name = self.new_with(*ty).name();
                format!("{name}?")
            }
            HirType::Array(ty, len) => {
                format!("[{len}]{}", self.new_with(*ty).name())
            }
            HirType::Vector(ty) => format!("[]{}", self.new_with(*ty).name()),

            HirType::Str => "str".to_string(),
            HirType::Reference { rf, generics } => {
                let name = self.new_with(*rf).name();
                let generics = {
                    let mut out = Vec::with_capacity(generics.len());
                    for generic in generics {
                        if *generic == DedupPoolId::new_null() {
                            break;
                        }
                        let ty = self.new_with(*generic).name();
                        out.push(ty);
                    }
                    out
                };
                if generics.is_empty() {
                    name
                } else {
                    format!("{name}<{}>", generics.join(","))
                }
            }
            HirType::GenericParam { name, .. } => self.hir.get_name(*name).to_string(),
            HirType::Style(s) => self.new_with(*s).name().to_string(),
            HirType::Struct(strukt) => {
                let name = self.new_with(*strukt).name();
                self.hir.get_name(name).to_string()
            }
            HirType::Function(func) => {
                let func = self.new_with(*func);
                let args = func
                    .arguments()
                    .iter()
                    .cloned()
                    .map(|arg| self.new_with(arg).name())
                    .collect::<Vec<String>>()
                    .join(",");
                let ret = self.new_with(func.return_type()).name();
                format!("func({args})->{ret}")
            }
            HirType::Tuple(tuple) => {
                let tuple = self.new_with(*tuple);
                let args = tuple
                    .fields()
                    .iter()
                    .cloned()
                    .map(|arg| self.new_with(arg).name())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({args})")
            }
            HirType::Component(component) => self.new_with(*component).name().to_string(),
        }
    }

    pub fn is_nullable(&self) -> Option<DedupPoolId<HirType>> {
        if let HirType::Nullable(inner) = self.hir.deref()[self.data] {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_vector(&self) -> Option<DedupPoolId<HirType>> {
        if let HirType::Vector(inner) = self.hir.deref()[self.data] {
            Some(inner)
        } else {
            None
        }
    }
    pub fn is_array(&self) -> Option<(DedupPoolId<HirType>, usize)> {
        if let HirType::Array(inner, size) = self.hir.deref()[self.data] {
            Some((inner, size))
        } else {
            None
        }
    }

    pub fn is_function(&self) -> Option<HirViewer<'_, DedupPoolId<FunctionType>>> {
        if let HirType::Function(f) = self.hir.deref()[self.data] {
            Some(self.new_with(f))
        } else {
            None
        }
    }
    pub fn is_struct(&self) -> Option<HirViewer<'_, DedupPoolId<StructType>>> {
        if let HirType::Struct(s) = self.hir.deref()[self.data] {
            Some(self.new_with(s))
        } else {
            None
        }
    }
    pub fn is_tuple(&self) -> Option<HirViewer<'_, DedupPoolId<TupleType>>> {
        if let HirType::Tuple(s) = self.hir.deref()[self.data] {
            Some(self.new_with(s))
        } else {
            None
        }
    }
    pub fn is_component(&self) -> Option<HirViewer<'_, DedupPoolId<ComponentType>>> {
        if let HirType::Component(s) = self.hir.deref()[self.data] {
            Some(self.new_with(s))
        } else {
            None
        }
    }
    pub fn is_style(&self) -> Option<HirViewer<'_, DedupPoolId<StyleType>>> {
        if let HirType::Style(s) = self.hir.deref()[self.data] {
            Some(self.new_with(s))
        } else {
            None
        }
    }
    ///Makes a dereference for this type. Since a type can be a reference to another, what this function does is to retrieve the concrete type with no references at all
    pub fn dereference(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        let mut data = self.data;
        while let HirType::Reference { rf, .. } = self.hir.deref()[data] {
            data = rf;
        }
        self.new_with(data)
    }
}

impl Deref for HirViewer<'_, DedupPoolId<HirType>> {
    type Target = HirType;
    fn deref(&self) -> &Self::Target {
        &self.hir.deref()[self.data]
    }
}
