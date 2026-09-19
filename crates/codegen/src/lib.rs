mod error;
mod lowerers;

use common::pool::DedupPoolId;
use slynx_hir::HirType;

pub use error::*;
pub use lowerers::{EnumLayout, LoweringState, TypeLowerer};

pub type TypeId = DedupPoolId<HirType>;
