use bitvec::vec::BitVec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserFlags {
    flags: BitVec,
}

#[repr(u32)]
pub enum ParserFlag {
    RequireSemicolon = 0,
    OnlySignatures = 1,
}

impl ParserFlags {
    pub fn new() -> Self {
        Self {
            flags: BitVec::new(),
        }
    }
    pub fn set_flag(&mut self, flag: ParserFlag) {
        self.flags.set(flag as usize, true);
    }

    pub fn remove_flag(&mut self, flag: ParserFlag) {
        self.flags.set(flag as usize, false);
    }

    pub fn has_flag(&self, flag: ParserFlag) -> bool {
        self.flags
            .get(flag as usize)
            .as_deref()
            .cloned()
            .unwrap_or(false)
    }
    pub fn reset(&mut self) {
        self.flags.clear();
    }
}
