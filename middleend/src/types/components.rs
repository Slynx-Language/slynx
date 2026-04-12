use smallvec::SmallVec;

use crate::IRTypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRSpecializedComponentType {
    Div,
    Text,
}

#[derive(Debug, Clone, Default)]
pub struct IRComponent {
    pub(crate) fields: SmallVec<[IRTypeId; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRComponentId(pub usize);

impl IRComponent {
    pub fn new() -> Self {
        Self {
            fields: SmallVec::new(),
        }
    }
    #[inline]
    pub fn insert_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }
}
