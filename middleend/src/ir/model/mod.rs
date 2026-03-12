///Module made specific to create the deffinitions and basic usages of the structures used on the IR

mod context;
mod instruction;
mod label;
mod ptr;
mod variable;

pub use context::*;
pub use instruction::*;
pub use label::*;
pub use ptr::*;
pub use variable::*;