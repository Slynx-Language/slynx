use crate::IRTypeId;
use common::SymbolPointer;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRSpecializedComponentType {
    Div,
    Text,
}

#[derive(Debug, Clone)]
pub struct IRComponent {
    pub(crate) name: SymbolPointer,
    pub(crate) fields: SmallVec<[IRTypeId; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRComponentId(pub usize);

impl IRComponent {
    pub fn new(name: SymbolPointer) -> Self {
        Self {
            name,
            fields: SmallVec::new(),
        }
    }
    #[inline]
    pub fn insert_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }

    #[inline]
    pub fn name(&self) -> SymbolPointer {
        self.name
    }

    #[inline]
    pub fn fields(&self) -> &[IRTypeId] {
        &self.fields
    }
}
