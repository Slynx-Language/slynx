use crate::{Component, IRComponent, IRType, IRViewer};

impl<'a> IRViewer<'a, Component> {
    pub fn raw_type(&'a self) -> &'a IRComponent {
        let comp = self.value();
        let ty = self.ir.types.get_type(comp.ty);
        let IRType::Component(c) = ty else {
            unreachable!("Type of component internally isnt a component? {ty:?}")
        };
        self.ir.types.get_component_type(c)
    }
}
