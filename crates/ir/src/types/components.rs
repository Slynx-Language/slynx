use std::hash::{Hash, Hasher};

use common::pool::DedupPoolId;
use smallvec::SmallVec;

use crate::{IRTypeId, SymbolPointer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRSpecializedComponentType {
    Div,
    Text,
}

#[derive(Debug, Clone)]
pub struct IRComponent {
    pub(crate) name: SymbolPointer,
    pub(crate) fields: SmallVec<[IRTypeId; 4]>,
    pub(crate) children: SmallVec<[IRTypeId; 4]>,
}

///A reference to some component on the IR
pub type IRComponentId = DedupPoolId<IRComponent>;

impl IRComponent {
    pub fn new(name: SymbolPointer) -> Self {
        Self {
            name,
            fields: SmallVec::new(),
            children: SmallVec::new(),
        }
    }
    #[inline]
    pub fn insert_field(&mut self, field: IRTypeId) {
        self.fields.push(field);
    }

    #[inline]
    pub fn insert_child(&mut self, field: IRTypeId) {
        self.children.push(field);
    }

    #[inline]
    pub fn name(&self) -> SymbolPointer {
        self.name
    }

    #[inline]
    pub fn fields(&self) -> &[IRTypeId] {
        &self.fields
    }

    pub fn children(&self) -> &[IRTypeId] {
        &self.children
    }
}

impl PartialEq for IRComponent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for IRComponent {}

impl Hash for IRComponent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
