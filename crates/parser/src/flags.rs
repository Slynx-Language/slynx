#[derive(Debug)]
pub struct BitVec {
    vec: Vec<u8>,
}
impl BitVec {
    pub fn new() -> Self {
        Self { vec: Vec::new() }
    }

    ///Calculates the index inside the vec, and the bit position to apply. It works with the following: let (index, bitpos) = calculate_position(n); self.vec[index] |= (b as usize) << bitpos;
    pub fn calculate_position(index: usize) -> (usize, usize) {
        let vecpos = index >> 3; //1 << 3 = 8, so this is the same as dividing by 8
        let bitpos = index & 7; //same as % 8;
        (vecpos, bitpos)
    }

    pub fn set(&mut self, index: usize, b: bool) {
        let (vecpos, bitpos) = Self::calculate_position(index);
        if self.vec.len() <= vecpos {
            self.vec.resize((vecpos * 2).max(1), 0);
        }
        let v = 1 << bitpos;
        if b {
            self.vec[vecpos] |= v;
        } else {
            self.vec[vecpos] &= !v;
        }
    }

    pub fn has(&self, index: usize) -> bool {
        let (vecpos, bitpos) = Self::calculate_position(index);
        self.vec
            .get(vecpos)
            .map(|byte| (byte & (1 << bitpos)) != 0)
            .unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }
}

#[derive(Debug)]
pub struct ParserFlags {
    flags: BitVec,
}

#[repr(u32)]
pub enum ParserFlag {
    RequireSemicolon = 0,
    OnlySignatures = 1,
    ComponentExpr = 2,
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
        self.flags.has(flag as usize)
    }
    pub fn reset(&mut self) {
        self.flags.clear();
    }
}
