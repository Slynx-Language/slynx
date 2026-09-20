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

impl std::fmt::Display for IRError {
    ///Formats the `IRError` into a human-readable message combining its kind
    ///and description, so consumers can surface it without re-encoding both
    ///enums by hand.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            IRErrorKind::InvalidType => "invalid type",
            IRErrorKind::InvalidLabelSwitch => "invalid label switch",
        };
        let description = match self.description {
            IRErrorDescription::NotAStruct => "value is not a struct",
            IRErrorDescription::InexistentLabel => "label does not exist",
            IRErrorDescription::SealedDescription => "label has already been sealed",
        };
        write!(f, "{kind}: {description}")
    }
}

impl std::error::Error for IRError {}
