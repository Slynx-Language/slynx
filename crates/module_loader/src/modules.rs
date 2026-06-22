use crate::{SourceLoader, SourceNode};

pub struct Modules {
    pub(crate) loader: SourceLoader,
    pub(crate) modules: Vec<SourceNode>,
}
