use swc_ecma_ast::Ident;

use crate::{
    compiler::js::WebCompiler,
    
};

use middleend::intermediate::{
        IntermediateRepr,
        context::{IntermediateContext, IntermediateContextType},
        id::{ContextHandle, PropId, VarId},
    };

#[derive(Debug)]
///A Function, also refered as 'context' is simply something that might have a scope to run something
pub struct JsFunction {
    pub handle: ContextHandle,
    pub variable_ids: Vec<VarId>,
    pub variables: Vec<Ident>,
    pub properties: Vec<PropId>,
}

impl JsFunction {
    pub fn new(handle: ContextHandle) -> Self {
        Self {
            handle,
            variables: Vec::new(),
            variable_ids: Vec::new(),
            properties: Vec::new(),
        }
    }
    pub fn create_variable(&mut self, id: VarId, name: &str) -> &Ident {
        self.variables.push(name.into());
        self.variable_ids.push(id);
        self.variables.last().unwrap()
    }
    pub fn create_property(&mut self, id: PropId, name: &str) -> &Ident {
        self.variables.push(name.into());
        self.properties.push(id);
        self.variables.last().unwrap()
    }
    pub fn retrieve_varname(&self, id: VarId) -> Option<&Ident> {
        self.variable_ids
            .iter()
            .rposition(|vid| *vid == id)
            .map(|idx| &self.variables[idx])
    }
}

impl WebCompiler {
    pub fn hoist_ctx(&mut self, ctx: &IntermediateContext, jscontext: &mut JsFunction) {
        match ctx.ty {
            IntermediateContextType::Component { .. } => {
                self.retrieve_next_component_name(jscontext.handle);
            }
            IntermediateContextType::Function { .. } => {
                self.retrieve_next_func_name(jscontext.handle);
            }
        }
    }

    pub fn compile_ctx(
        &mut self,
        ctx: &IntermediateContext,
        ir: &IntermediateRepr,
        jsfunc: ContextHandle,
    ) {
        match &ctx.ty {
            IntermediateContextType::Component { properties, .. } => {
                self.compile_component(properties, ctx, ir, jsfunc);
            }
            IntermediateContextType::Function { .. } => self.compile_function(ctx, ir, jsfunc),
        }
    }
}
