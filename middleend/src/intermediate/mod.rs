pub mod context;
pub mod expr;
pub mod id;
pub mod node;
pub mod string;
pub mod types;
use std::collections::HashMap;

use crate::intermediate::{
    context::{IntermediateContext, IntermediateContextType},
    expr::IntermediateExpr,
    id::{ContextHandle, TyId, ValueId, VarId},
    node::{IntermediateInstruction, IntermediatePlace},
    string::StringPool,
    types::IntermediateType,
};
use frontend::hir::{
    DeclarationId, PropertyId, TypeId, VariableId,
    definitions::{
        ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind, HirExpression,
        HirExpressionKind, HirStatement, HirStatementKind, SpecializedComponent,
    },
    types::{HirType, TypesModule},
};
#[derive(Debug, Default)]
/// Struct used to represent the intermediate representation of Slynx. Will be used when being compiled. This contains the monomorfization of types
/// and flat values of everything in the source code. It can be understood as the context itself of the IR
pub struct IntermediateRepr {
    pub contexts: Vec<IntermediateContext>,
    pub context_mapping: HashMap<TyId, ContextHandle>,
    pub types_mapping: HashMap<TyId, IntermediateType>,
    pub vars: HashMap<VariableId, VarId>,
    pub props: HashMap<PropertyId, VarId>,
    ///Mapping of high level types to low level types
    pub types: HashMap<TypeId, TyId>,
    pub functions_mapping: HashMap<DeclarationId, ContextHandle>,
    pub strings: StringPool,
}

impl IntermediateRepr {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn active_context(&mut self) -> &mut IntermediateContext {
        let len = self.contexts.len();
        &mut self.contexts[len - 1]
    }

    /// Generates a new expression and returns it's id on the current context
    fn generate_expr(&mut self, expr: HirExpression) -> ValueId {
        match expr.kind {
            HirExpressionKind::Bool(b) => {
                self.active_context().insert_expr(IntermediateExpr::bool(b))
            }
            HirExpressionKind::StringLiteral(s) => {
                let handle = self.strings.insert_string(&s);
                self.active_context()
                    .insert_expr(IntermediateExpr::str(handle))
            }
            HirExpressionKind::Int(int) => self
                .active_context()
                .insert_expr(IntermediateExpr::int(int)),
            HirExpressionKind::Float(float) => self
                .active_context()
                .insert_expr(IntermediateExpr::float(float)),

            HirExpressionKind::Binary { lhs, op, rhs } => {
                let lhs = self.generate_expr(*lhs);
                let rhs = self.generate_expr(*rhs);
                self.active_context()
                    .insert_expr(IntermediateExpr::bin(lhs, rhs, op))
            }
            HirExpressionKind::Identifier(name) => {
                let varid = self
                    .vars
                    .get(&name)
                    .or_else(|| self.props.get(&PropertyId::from_raw(name.as_raw())))
                    .cloned()
                    .unwrap();
                self.active_context()
                    .insert_expr(IntermediateExpr::identifier(varid))
            }
            HirExpressionKind::FunctionCall { name, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.generate_expr(arg))
                    .collect();
                let func_handle = *self.functions_mapping.get(&name).expect("A função deveria ser referenciada, isso tá sendo temporario mas o problema provavel uqe é hoisting");
                self.active_context()
                    .insert_expr(IntermediateExpr::functioncall(func_handle, args))
            }
            HirExpressionKind::Component { name, values, .. } => {
                let name = *self.types.get(&name).unwrap();
                let eidx = self.generate_child(name, values);
                if matches!(
                    self.active_context().ty,
                    IntermediateContextType::Component { .. }
                ) {
                    self.active_context().insert_child(eidx); //child index
                };
                eidx
            }
            HirExpressionKind::Specialized(spec) => {
                let sidx = self.generate_specialized(spec);
                if matches!(
                    self.active_context().ty,
                    IntermediateContextType::Component { .. }
                ) {
                    self.active_context().insert_child(sidx);
                }
                sidx
            }
            HirExpressionKind::Object { name, fields } => {
                let indexes = fields
                    .into_iter()
                    .map(|expr| self.generate_expr(expr))
                    .collect();
                let name = *self.types.get(&name).unwrap();
                self.active_context()
                    .insert_expr(IntermediateExpr::strct(name, indexes))
            }
            HirExpressionKind::FieldAccess { expr, field_index } => {
                let expr = self.generate_expr(*expr);
                self.active_context()
                    .insert_expr(IntermediateExpr::field(expr, field_index))
            }
        }
    }

    /// Creates a new child on the current context and returns the component expression and the child id
    fn generate_child(&mut self, name: TyId, values: Vec<ComponentMemberDeclaration>) -> ValueId {
        let mut props = Vec::new();
        let mut children = Vec::new();

        for value in values {
            match value {
                ComponentMemberDeclaration::Property { index, value, .. } => {
                    let out = value.map(|value| self.generate_expr(value));
                    for _ in 0..index - props.len() {
                        props.push(None);
                    }
                    props.push(out);
                }
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    let name = *self.types.get(&name).unwrap();
                    let idx = self.generate_child(name, values);
                    children.push(idx);
                }
                ComponentMemberDeclaration::Specialized(spec) => {
                    let idx = self.generate_specialized(spec);
                    children.push(idx);
                }
            }
        }
        let name = *self.context_mapping.get(&name).unwrap();
        //expr index
        let eidx = self
            .active_context()
            .insert_expr(IntermediateExpr::component(name, props, children));
        self.active_context().insert_child(eidx);
        eidx
    }

    fn generate_specialized(&mut self, spec: SpecializedComponent) -> ValueId {
        let expr = match spec {
            SpecializedComponent::Text { text } => {
                let txt = self.generate_expr(*text);
                IntermediateExpr::native_text(txt, Vec::new())
            }
            SpecializedComponent::Div { children } => {
                let mut props = Vec::new();
                let children = children
                    .into_iter()
                    .filter_map(|v| match v {
                        ComponentMemberDeclaration::Child { name, values, .. } => {
                            let name = *self.types.get(&name).unwrap();
                            Some(self.generate_child(name, values))
                        }
                        ComponentMemberDeclaration::Property { index, value, .. } => {
                            let out = value.map(|v| self.generate_expr(v));
                            for _ in 0..index - props.len() {
                                props.push(None);
                            }
                            out
                        }
                        ComponentMemberDeclaration::Specialized(spec) => {
                            Some(self.generate_specialized(spec))
                        }
                    })
                    .collect();
                IntermediateExpr::native_rect(children, props)
            }
        };
        let sidx = self.active_context().insert_expr(expr);
        self.active_context().insert_child(sidx);
        sidx
    }

    /// Creates a new component with the provided informations. Returns the component id and it's type id
    fn generate_component(
        &mut self,
        props: Vec<ComponentMemberDeclaration>,
        tys: &[IntermediateType],
    ) -> ContextHandle {
        let handle = ContextHandle(self.contexts.len());
        let expr = IntermediateContext::new_component(handle);

        self.contexts.push(expr);
        for (idx, prop) in props.into_iter().enumerate() {
            match prop {
                ComponentMemberDeclaration::Property { value, id, .. } => {
                    let default_value = value.map(|value| self.generate_expr(value));

                    self.active_context()
                        .insert_property(default_value, tys[idx].clone());
                    self.props.insert(id, VarId::new());
                }
                ComponentMemberDeclaration::Specialized(spec) => {
                    self.generate_specialized(spec);
                }
                ComponentMemberDeclaration::Child { name, values, .. } => {
                    let name = *self.types.get(&name).unwrap();
                    self.generate_child(name, values);
                }
            }
        }
        handle
    }

    pub fn generate_var(&mut self, name: VarId, value: HirExpression) -> usize {
        let idx = self.generate_expr(value);
        self.active_context()
            .insert_instruction(IntermediateInstruction::alloc(name))
            .unwrap();
        self.active_context()
            .insert_instruction(IntermediateInstruction::mov(
                IntermediatePlace::Local(name),
                idx,
            ))
            .unwrap()
    }

    fn generate_place(&mut self, lhs: HirExpression) -> IntermediatePlace {
        match lhs.kind {
            HirExpressionKind::Identifier(id) => {
                IntermediatePlace::Local(*self.vars.get(&id).unwrap())
            }
            HirExpressionKind::FieldAccess { field_index, expr } => {
                let expr = self.generate_expr(*expr);
                let id = VarId::new();
                self.active_context()
                    .insert_instruction(IntermediateInstruction::alloc(id))
                    .unwrap();
                self.active_context()
                    .insert_instruction(IntermediateInstruction::mov(
                        IntermediatePlace::Local(id),
                        expr,
                    ));
                IntermediatePlace::Field {
                    parent: id,
                    field: field_index,
                }
            }
            _ => panic!("Unsupported expression type for place generation: {lhs:?}"),
        }
    }

    pub fn generate_assign(&mut self, lhs: HirExpression, value: HirExpression) -> usize {
        let idx = self.generate_expr(value);
        let place = self.generate_place(lhs);
        self.active_context()
            .insert_instruction(IntermediateInstruction::mov(place, idx))
            .unwrap()
    }
    /// Creates a new function with the provided `name` `id` and `Statements` and returns its id
    pub fn generate_function(
        &mut self,
        args: Vec<(IntermediateType, VarId)>,
        ret: IntermediateType,
        statements: Vec<HirStatement>,
        name: String,
    ) -> ContextHandle {
        let handle = ContextHandle(self.contexts.len());
        let ctx = IntermediateContext::new_function(handle, name, args, ret);

        self.contexts.push(ctx);
        for statement in statements {
            match statement.kind {
                HirStatementKind::Assign { lhs, value } => {
                    self.generate_assign(lhs, value);
                }
                HirStatementKind::Variable { name, value, .. } => {
                    let v = VarId::new();
                    self.vars.insert(name, v);
                    self.generate_var(v, value);
                }
                HirStatementKind::Expression { expr } => {
                    self.generate_expr(expr);
                }
                HirStatementKind::Return { expr } => {
                    let id = self.generate_expr(expr);
                    self.active_context()
                        .insert_instruction(node::IntermediateInstruction::ret(id));
                }
                _ => {}
            };
        }
        handle
    }

    pub fn generate(&mut self, decls: Vec<HirDeclaration>, module: TypesModule) {
        for decl in decls {
            match decl.kind {
                HirDeclarationKind::Object => {
                    let HirType::Struct { fields } = module.get_type_from_ref(&decl.ty) else {
                        unreachable!("Type of a object declaration should be 'struct'");
                    };
                    let ty = self.retrieve_complex(fields.as_slice(), &module);
                    let tyid = TyId::new();
                    self.types.insert(decl.ty, tyid);
                    self.types_mapping.insert(tyid, ty); // Convert DeclarationId to HirId
                }
                HirDeclarationKind::Function {
                    statements,
                    name,
                    args,
                } => {
                    let HirType::Function {
                        args: tys,
                        return_type,
                    } = module.get_type(&decl.ty)
                    else {
                        unreachable!("Type of function decl should be function")
                    };

                    let argsty = tys
                        .iter()
                        .zip(&args)
                        .map(|(ty, id)| {
                            let v = VarId::new();
                            self.vars.insert(*id, v);
                            (self.get_type(ty, &module), v)
                        })
                        .collect();
                    let ret = self.get_type(return_type, &module);
                    let handle = self.generate_function(argsty, ret, statements, name);
                    let ty = TyId::new();
                    self.functions_mapping.insert(decl.id, handle);
                    self.context_mapping.insert(ty, handle);
                }
                HirDeclarationKind::ComponentDeclaration { props } => {
                    let mut tys = Vec::new();
                    let HirType::Component { props: decl_ty } = module.get_type(&decl.ty) else {
                        unreachable!("Type of component decl should be component");
                    };
                    for (_, _, ty) in decl_ty {
                        let typ = self.get_type(ty, &module);
                        tys.push(typ);
                    }

                    let handle = self.generate_component(props, &tys); // Convert DeclarationId to HirId
                    let ty = TyId::new();
                    self.types.insert(decl.ty, ty);
                    self.context_mapping.insert(ty, handle);
                    self.types_mapping
                        .insert(ty, IntermediateType::Complex(tys));
                    // Convert DeclarationId to HirId
                }
            }
        }
    }
}
