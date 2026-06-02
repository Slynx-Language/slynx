use crate::{IRTypeId, IRTypes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[repr(u16)]
pub enum StyleProperty {
    BackgroundColor = 0,
    ForegroundColor = 1,
}

impl StyleProperty {
    /// Resolve stylesheet textual name -> StyleProperty
    pub fn from_name(name: &str) -> Self {
        match name {
            "backgroundColor" => Self::BackgroundColor,
            "foregroundColor" => Self::ForegroundColor,
            _ => panic!(
                "Property named as {name} DOES NOT EXIST, and should be treated on previous phases. More specifically, HIR"
            ),
        }
    }

    /// Numeric STYLES_TABLE code
    #[inline]
    pub fn code(self) -> u16 {
        self as u16
    }

    /// IR field type for this style property
    pub fn ir_type(self, types: &IRTypes) -> IRTypeId {
        match self {
            Self::BackgroundColor | Self::ForegroundColor => {
                // Color -> i32 placeholder
                types.int_type()
            }
        }
    }
}

impl std::fmt::Display for StyleProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackgroundColor => write!(f, "BACKGROUND_COLOR"),
            Self::ForegroundColor => write!(f, "FOREGROUND_COLOR"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_styles_table() {
        assert_eq!(StyleProperty::BackgroundColor.code(), 0);
        assert_eq!(StyleProperty::ForegroundColor.code(), 1);
    }
}
