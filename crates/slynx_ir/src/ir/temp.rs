use std::collections::HashMap;

use slynx_hir::{DeclarationId, TypeId, VariableId, model::HirDeclaration, modules::TypesModule};
use smallvec::SmallVec;

use crate::{
    Component, IRError, IRTypeId,
    ir::model::{Context, IRPointer, Label, Value},
};

pub struct AuxiliaryStyle {
    pub init_func: IRPointer<Context, 1>,
    pub apply_func: IRPointer<Context, 1>,
    pub strct: IRTypeId,
}

///Data to save Component information, such as its IRPointer, and values for the default fields
pub struct TempComponentData {
    pub ptr: IRPointer<Component, 1>,
    ///Vector mapping the Nth field -> Default value, of this component. Its u8 because, seriously no one is going to make a component with more than 255 properties
    pub default_properties: SmallVec<[(IRPointer<Value, 1>, u8); 4]>,
}

///Temporary IR Data to be able to map the HIR contents to the IR contents that are being generated. This should only live during `generate` function of
/// slynx ir
pub struct TempIRData<'a> {
    types_module: &'a TypesModule,
    /// The full HIR declaration list, used for lookups during IR generation.
    pub hir: &'a [HirDeclaration],
    ///Maps HIR types to IR types
    types_mapping: HashMap<TypeId, IRTypeId>,
    ///Maps HIR functions to IR functions
    functions: HashMap<DeclarationId, IRPointer<Context, 1>>,
    ///Maps HIR components to IR functions
    components: HashMap<DeclarationId, TempComponentData>,
    ///The current function being generated
    current_function: IRPointer<Context, 1>,
    ///The current lavel that is being generated on the current function
    current_label: IRPointer<Label, 1>,
    ///The variables on the current function
    variables: Vec<(VariableId, IRPointer<Value, 1>)>,
    ///The arguments of the current variable ID
    args: Vec<VariableId>,
    styles: HashMap<DeclarationId, AuxiliaryStyle>,
    init_functions: HashMap<DeclarationId, IRPointer<Context, 1>>,
}

impl<'a> TempIRData<'a> {
    pub fn new(types_module: &'a TypesModule, hir: &'a [HirDeclaration]) -> Self {
        Self {
            types_module,
            hir,
            types_mapping: HashMap::new(),
            functions: HashMap::new(),
            current_function: IRPointer::null(),
            components: HashMap::new(),
            current_label: IRPointer::null(),
            args: Vec::new(),
            variables: Vec::new(),
            styles: HashMap::new(),
            init_functions: HashMap::new(),
        }
    }

    #[inline]
    pub fn types_module(&self) -> &TypesModule {
        self.types_module
    }

    ///Maps the provided `hty`(hir type) to the provided `ity`(ir type)
    #[inline]
    pub fn define_type(&mut self, hty: TypeId, ity: IRTypeId) {
        self.types_mapping.insert(hty, ity);
    }

    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn map_function(&mut self, fid: DeclarationId, func: IRPointer<Context, 1>) {
        self.functions.insert(fid, func);
    }

    #[inline]
    pub fn map_style(&mut self, sid: DeclarationId, data: AuxiliaryStyle) {
        self.styles.insert(sid, data);
    }

    #[inline]
    pub fn get_style(&self, sid: DeclarationId) -> Option<&AuxiliaryStyle> {
        self.styles.get(&sid)
    }

    #[inline]
    ///Retrieves the apply function context for the given stylesheet declaration ID
    pub fn get_style_apply_function(&self, sid: DeclarationId) -> Option<IRPointer<Context, 1>> {
        self.styles.get(&sid).map(|s| s.apply_func)
    }

    #[inline]
    ///Retrieves the IR struct type for the given stylesheet declaration ID
    pub fn get_style_struct_type(&self, sid: DeclarationId) -> Option<IRTypeId> {
        self.styles.get(&sid).map(|s| s.strct)
    }

    #[inline]
    ///Retrieves the init function for the given stylesheet declaration ID
    pub fn get_style_init_function(&self, sid: DeclarationId) -> Option<IRPointer<Context, 1>> {
        self.styles.get(&sid).map(|s| s.init_func)
    }

    #[inline]
    pub fn map_init_function(&mut self, did: DeclarationId, func: IRPointer<Context, 1>) {
        self.init_functions.insert(did, func);
    }

    #[inline]
    pub fn get_init_function(&self, did: DeclarationId) -> IRPointer<Context, 1> {
        self.init_functions
            .get(&did)
            .copied()
            .expect("Init function not found for component declaration")
    }
    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn map_component(&mut self, fid: DeclarationId, comp: IRPointer<Component, 1>) {
        self.components.insert(
            fid,
            TempComponentData {
                ptr: comp,
                default_properties: SmallVec::new(),
            },
        );
    }
    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn get_component(&self, fid: DeclarationId) -> &TempComponentData {
        self.components
            .get(&fid)
            .expect("For some reason the provided Function Id is not declared")
    }

    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn get_component_mut(&mut self, fid: DeclarationId) -> &mut TempComponentData {
        self.components
            .get_mut(&fid)
            .expect("For some reason the provided Function Id is not declared")
    }
    #[inline]
    ///Maps the provided `fid`(hir function id) to the provided `func`(ir function)
    pub fn get_function(&self, fid: DeclarationId) -> IRPointer<Context, 1> {
        self.functions
            .get(&fid)
            .cloned()
            .expect("For some reason the provided Function Id is not declared")
    }
    #[inline]
    ///Gets the IR type for the provided `ty`(hir type)
    pub fn get_type(&self, ty: TypeId) -> Result<IRTypeId, IRError> {
        if let Some(ty) = self.types_mapping.get(&ty) {
            Ok(*ty)
        } else {
            Err(IRError::IRTypeNotRecognized(ty))
        }
    }

    #[inline]
    ///Retrieves the current label on the current function
    pub fn current_label(&self) -> IRPointer<Label, 1> {
        self.current_label
    }
    #[inline]
    ///Retrieves the current label on the current function
    pub fn set_current_label(&mut self, label: IRPointer<Label, 1>) {
        self.current_label = label
    }
    #[inline]
    pub fn set_function_args(&mut self, args: &[VariableId], ptr: IRPointer<Value>) {
        self.variables.clear();
        for (idx, arg) in args.iter().enumerate() {
            self.add_variable(*arg, ptr.ptr_to(idx));
        }
    }

    #[inline]
    ///Sets the current function being generated and resets the variables
    pub fn current_function(&self) -> IRPointer<Context, 1> {
        self.current_function
    }
    #[inline]
    ///Sets the current function being generated and resets the variables
    pub fn set_current_function(&mut self, func: IRPointer<Context, 1>) {
        self.variables.clear();
        self.args.clear();
        self.current_function = func;
    }

    ///Adds the given value mapped to the given `id`
    pub fn add_variable(&mut self, id: VariableId, value: IRPointer<Value, 1>) {
        self.variables.push((id, value));
    }

    #[inline]
    ///Gets the variable that matches the provided `id` on the current function
    pub fn get_variable(&self, id: VariableId) -> Option<IRPointer<Value, 1>> {
        self.variables
            .iter()
            .find_map(|v| if v.0 == id { Some(v.1) } else { None })
    }
}
