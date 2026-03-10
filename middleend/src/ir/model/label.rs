use super::{IRPointer, instruction::Instruction};

///A label is a named 'piece' of block that has got instructions and can be used to determine values
pub struct Label {
    ///The instructions this label has got. The max limit due to the IRPointer is about 65k instructions per label
    instruction: IRPointer<Instruction>,
}
