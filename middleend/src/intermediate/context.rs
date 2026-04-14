use std::{collections::HashMap, ops::Index};

use crate::intermediate::{
    expr::IntermediateExpr,
    id::{ContextHandle, PropId, ValueId, VarId},
    node::IntermediateInstruction,
    types::IntermediateType,
};

#[derive(Debug)]
///A Intermediate property used to bind an id to it's default value on the current context
pub struct IntermediateProperty {
    pub id: PropId,
    pub ty: IntermediateType,
    pub default_value: Option<ValueId>,
}

#[derive(Debug)]
pub enum IntermediateContextType {
    Function {
        name: String,
        instructions: Vec<IntermediateInstruction>,
        args: Vec<(IntermediateType, VarId)>,
        ret: IntermediateType,
    },
    Component {
        properties: Vec<IntermediateProperty>,
        children: Vec<ValueId>,
    },
}

#[derive(Debug)]
pub struct IntermediateContext {
    pub id: ContextHandle,
    pub exprs: Vec<IntermediateExpr>,
    ///A vector of pointer to the expressions
    pub vars: Vec<VarId>,
    ///Maps some name to an expression
    pub names: HashMap<VarId, usize>,
    pub ty: IntermediateContextType,
}

impl IntermediateContext {
    pub fn new_function(
        id: ContextHandle,
        name: String,
        args: Vec<(IntermediateType, VarId)>,
        ret: IntermediateType,
    ) -> Self {
        Self {
            id,
            exprs: Vec::new(),
            vars: Vec::new(),
            names: HashMap::new(),
            ty: IntermediateContextType::Function {
                instructions: Vec::new(),
                name,
                args,
                ret,
            },
        }
    }
    pub fn new_component(id: ContextHandle) -> Self {
        Self {
            id,
            exprs: Vec::new(),
            vars: Vec::new(),
            names: HashMap::new(),
            ty: IntermediateContextType::Component {
                properties: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    pub fn allocate(&mut self, id: VarId) -> usize {
        let out = self.vars.len();
        self.insert_instruction(IntermediateInstruction::alloc(id));
        self.vars.push(id);
        out
    }

    ///Inserts the provided `expr` on this context and returns it's id
    pub fn insert_expr(&mut self, expr: IntermediateExpr) -> ValueId {
        let id = ValueId::from_raw(self.exprs.len() as u64);
        self.exprs.push(expr);
        id
    }
    ///Adds on this context instructions, the given `instr`.
    ///Returns the ID of the instruction. Returns None if this isn't a function
    pub fn insert_instruction(&mut self, instr: IntermediateInstruction) -> Option<usize> {
        match &mut self.ty {
            IntermediateContextType::Function { instructions, .. } => {
                let id = instructions.len();
                instructions.push(instr);
                Some(id)
            }
            _ => None,
        }
    }
    ///Inserts a new property on this component and returns it's property child index.
    ///Returns None if this isn't an component
    pub fn insert_property(
        &mut self,
        value: Option<ValueId>,
        ty: IntermediateType,
    ) -> Option<PropId> {
        match &mut self.ty {
            IntermediateContextType::Component { properties, .. } => {
                let prop = PropId::from_raw(properties.len() as u64);
                properties.push(IntermediateProperty {
                    id: prop,
                    ty,
                    default_value: value,
                });
                Some(prop)
            }
            _ => None,
        }
    }

    ///Creates a new child that value is the provided `expr` in this context. Returns its index if this is a context, else None, and does nothing
    pub fn insert_child(&mut self, expr: ValueId) -> Option<ValueId> {
        if let IntermediateContextType::Component {
            ref mut children, ..
        } = self.ty
        {
            let id = children.len();
            children.push(expr);
            Some(ValueId::from_raw(id as u64))
        } else {
            None
        }
    }
}

impl Index<ValueId> for Vec<IntermediateExpr> {
    type Output = IntermediateExpr;
    fn index(&self, index: ValueId) -> &Self::Output {
        &self[index.as_raw() as usize]
    }
}
