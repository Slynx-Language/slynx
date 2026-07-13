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
            _ if let Some(func) = self.is_function() => {
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
            _ if let Some(tuple) = self.is_tuple() => {
                let args = tuple
                    .fields()
                    .iter()
                    .cloned()
                    .map(|arg| self.new_with(arg).name())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({args})")
            }
            _ if let Some(strukt) = self.is_struct() => {
                self.hir.get_name(strukt.name()).to_string()
            }
            _ if let Some(component) = self.is_component() => component.name().to_string(),
            t => unreachable!("Type {t:?} is not be able to have a name"),
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
        while let HirType::Reference { rf, .. } = self.hir.deref()[self.data] {
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
