mod error;
mod lowerers;

use slynx_hir::term::TermId;

pub use error::*;
pub use lowerers::{EnumLayout, LoweringState, TypeLowerer};

pub type TypeId = TermId;
