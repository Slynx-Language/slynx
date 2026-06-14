use dashmap::DashMap;

use crate::DeclarationId;

#[derive(Debug, Default)]
///A struct to map intrinsic functions, objects, etc
pub struct LangItems {
    declarations: DashMap<String, DeclarationId>,
}

impl LangItems {
    pub fn new() -> Self {
        Self {
            declarations: DashMap::new(),
        }
    }
    pub fn register(&self, name: &str, id: DeclarationId) {
        if self.declarations.insert(name.to_string(), id).is_some() {
            panic!("Double delcarations on registering an intrinsics declaration with name {name}");
        }
    }
    pub fn try_get(&self, name: &str) -> Option<DeclarationId> {
        self.declarations.get(name).map(|v| *v)
    }
    pub fn get(&self, name: &str) -> Result<DeclarationId, String> {
        self.try_get(name).ok_or_else(|| format!("intrinsic '{name}' is not registered"))
    }
}
