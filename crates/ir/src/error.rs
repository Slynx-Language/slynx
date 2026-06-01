#[derive(Debug)]
pub enum IRErrorKind {
    InvalidType,
    InvalidLabelSwitch,
}

#[derive(Debug)]
pub enum IRErrorDescription {
    NotAStruct,
    InexistentLabel,
    SealedDescription,
}
#[derive(Debug)]
pub struct IRError {
    pub kind: IRErrorKind,
    pub description: IRErrorDescription,
}

impl IRError {
    pub fn new(kind: IRErrorKind, description: IRErrorDescription) -> Self {
        Self { kind, description }
    }
}
