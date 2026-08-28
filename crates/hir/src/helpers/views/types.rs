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
            HirType::ImutableRef(ty) => format!("&{}", self.new_with(*ty).name()),
            HirType::MutableRef(ty) => format!("&mut {}", self.new_with(*ty).name()),

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

    ///Returns the concrete type of this type. If this type is a reference type(a type that maps to another type), this function will find the concrete type, and if the concrete type is a reference(&T/&mut T), this function will retrieve T
    ///Its then different from [`dereference`] which will return the type by remapping it, but yet making &T/&mut T a possible return case, where this one gets the concrete T type being used. This is mainly useful for checks where
    ///you do wanna see the concrete type that is being dealed, even though its abstracted by something, such, as said above, &T.
    ///Same thing to track nullable types.
    ///
    ///```
    ///let strukt_type = hir.create_struct_type();
    ///let ty = hir.create_type(HirType::Nullable(strukt_type));
    ///let type_view = type_view.concrete_type();
    ///assert_eq!(type_view.data, strukt_type);
    ///```
    ///
    pub fn concrete_type(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        let mut data = self.data;
        while let HirType::Nullable(rf)
        | HirType::ImutableRef(rf)
        | HirType::MutableRef(rf)
        | HirType::Reference { rf, .. } = self.hir.deref()[data]
        {
            data = rf
        }
        self.new_with(data)
    }

    pub fn is_ref(&self) -> bool {
        matches!(
            self.hir.deref()[self.data],
            HirType::ImutableRef { .. } | HirType::MutableRef { .. }
        )
    }
    pub fn is_imutable_ref(&self) -> Option<HirViewer<'_, DedupPoolId<HirType>>> {
        if let HirType::ImutableRef(inner) = self.hir.deref()[self.data] {
            Some(self.new_with(inner))
        } else {
            None
        }
    }
    pub fn is_mutable_ref(&self) -> Option<HirViewer<'_, DedupPoolId<HirType>>> {
        if let HirType::MutableRef(inner) = self.hir.deref()[self.data] {
            Some(self.new_with(inner))
        } else {
            None
        }
    }
}

impl Deref for HirViewer<'_, DedupPoolId<HirType>> {
    type Target = HirType;
    fn deref(&self) -> &Self::Target {
        &self.hir.deref()[self.data]
    }
}

impl std::fmt::Display for HirViewer<'_, DedupPoolId<HirType>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
