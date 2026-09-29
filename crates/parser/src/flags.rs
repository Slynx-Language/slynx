use bitflags::bitflags;

bitflags! {
    #[derive(Debug,Clone, Copy)]
    pub struct ParserFlags: u64 {
        const REQUIRE_SEMICOLON = 1 << 0;
        const ONLY_SIGNATURES = 1 << 1;
        const COMPONENT_EXPR= 1 << 2;
    }
}

impl std::default::Default for ParserFlags {
    fn default() -> Self {
        Self::REQUIRE_SEMICOLON
    }
}
