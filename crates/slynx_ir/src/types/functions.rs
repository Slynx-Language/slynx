use smallvec::SmallVec;

use crate::IRTypeId;

#[derive(Debug)]
pub struct IRFunction {
    args: SmallVec<[IRTypeId; 8]>,
    ret: IRTypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
///A reference to some struct on the IR
pub struct IRFunctionId(pub usize);

impl IRFunction {
    ///Creates a new function type with the provided `args` and `ret` return type
    pub fn new(args: &[IRTypeId], ret: IRTypeId) -> Self {
        Self {
            args: args.into(),
            ret,
        }
    }

    #[inline]
    ///Inserts the provided `args` on the args of this function
    pub fn insert_arg_types(&mut self, args: &[IRTypeId]) {
        self.args.extend_from_slice(args);
    }
    #[inline]
    ///Sets the return type of this function to be the provided `ret`
    pub fn set_return_type(&mut self, ret: IRTypeId) {
        self.ret = ret;
    }

    #[inline]
    ///Returns the arguments of this function
    pub fn get_args(&self) -> &[IRTypeId] {
        &self.args
    }

    #[inline]
    ///Returns the return type of this function
    pub fn get_return_type(&self) -> IRTypeId {
        self.ret
    }
}
