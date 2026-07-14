pub(crate) mod attributes;
pub(crate) mod component;
mod expression;
mod function;
pub(crate) mod styles;
mod work_channel;
use std::{cell::RefCell, ops::Deref};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};

use crossbeam_channel::select;
use dashmap::{DashMap, DashSet};
use module_loader::{ASTType, ASTTypeKind, FileId, Modules};
use slynx_parser::{
    ASTStatement, ComponentDeclaration, ComponentMemberKind, FuncDeclaration,
    StaticDeclaration, Type,
};

use crate::{
    ComponentId, ComponentMemberDeclaration, DeclarationId, HIRError, HirComponentDeclaration,
    HirFunctionDeclaration, HirObjectDeclaration, HirStatement, HirStaticDeclaration, HirType,
    Result, SlynxHir, SymbolPointer, VariableId,
    builders::{
        expression::ExpressionBuildResult, function::HirFunctionBuilder, work_channel::WorkChannel,
    },
    context::HirSymbol,
    helpers::Visible,
};

pub struct PendingSignatures<'a> {
    /// Signature resolution state per component (by (FileId, SymbolPointer)).
    pub signatures_in_progress: &'a DashSet<(FileId, SymbolPointer)>,
    pub signature_stack: &'a RefCell<Vec<(FileId, SymbolPointer)>>,
}

///A Node represents a file that is being compiled on the HIR. It's just a view over the Hir and AST to properly read data from the ast from the `entry` file
pub struct HirNode<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    pub(crate) pendings: PendingSignatures<'a>,
    ///The ID of the file that we are reading
    pub(crate) entry: FileId,
}

impl<'a> Deref for HirNode<'a> {
    type Target = Modules<'a>;
    fn deref(&self) -> &Self::Target {
        self.modules
    }
}

pub(crate) struct PendantBody<'a> {
    func_id: DeclarationId<HirFunctionDeclaration>,
    body: &'a [Spanned<DedupPoolId<ASTStatement>>],
    argument_names: Vec<SymbolPointer>,
}

pub(crate) struct PendantComponent<'a> {
    owner: DeclarationId<HirComponentDeclaration>,
    component: &'a ComponentDeclaration,
}

pub struct HirQueueBuilder<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    pub(crate) bodies: WorkChannel<PendantBody<'a>>,
    pub(crate) statics: WorkChannel<()>,
    #[allow(clippy::type_complexity)]
    pub(crate) resolved_bodies: DashMap<
        DeclarationId<HirFunctionDeclaration>,
        (Vec<Spanned<PoolId<HirStatement>>>, Vec<VariableId>),
    >,
    pub(crate) resolved_components:
        DashMap<DeclarationId<HirComponentDeclaration>, Vec<ComponentMemberDeclaration>>,
    pub(crate) components: WorkChannel<PendantComponent<'a>>,

    /// Signature resolution state per component (by (FileId, SymbolPointer)).
    pub signatures_in_progress: DashSet<(FileId, SymbolPointer)>,
    /// Body resolution state per component.
    pub bodies_in_progress: DashSet<ComponentId>,
    /// Stack for cycle-detection error chains during signature resolution.
    /// Single-threaded for now; see component-generation.md §8.
    // TODO(threading): replace with thread-local or DashMap<ThreadId, Vec<...>> when Rayon lands.
    pub signature_stack: RefCell<Vec<(FileId, SymbolPointer)>>,
}

impl HirNode<'_> {
    ///Finds the Hir type for the given `ty` and what file contains it if theres some. The given `file` is the file id where the given `ty` was generated at
    fn find_type(
        &self,
        ty: Spanned<DedupPoolId<Type>>,
    ) -> Result<(FileId, DedupPoolId<HirType>)> {
        let real = self.modules.get_type(ty.data);

        if let Some(ty) = self
            .modules
            .find_type_inside_module(self.entry, real.name())
        {
            let id = match ty.content {
                ASTTypeKind::Builtin(builtin) => self.hir.create_type(builtin.into()),
                ASTTypeKind::Alias(alias) => {
                    return self.find_type(alias.target);
                }
                ASTTypeKind::Struct(s) => {
                    let struct_name = self.type_name(s.name.data);
                    let fields = s
                        .fields
                        .iter()
                        .map(|field| {
                            let field_name = field.name.data.name;
                            let field_ty = field.name.data.kind;
                            let (_, type_id) = self.find_type(field_ty)?;

                            Ok(Visible::new(field.visibility, (field_name, type_id)))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let struct_ty = self.hir.create_struct_type(struct_name, fields, Vec::new());
                    // Register a HirObjectDeclaration so the codegen's
                    // hoist_declarations can create an IR struct for this type.
                    let file = self.hir.get_or_create_file(ty.owner);
                    let already = file
                        .declarations
                        .objects
                        .iter()
                        .any(|d| d.name == struct_name);
                    if !already {
                        file.create_object(HirObjectDeclaration {
                            name: struct_name,
                            ty: struct_ty,
                            visibility: s.visibility,
                            external: s.external,
                            attributes: Vec::new(),
                        });
                    }
                    struct_ty
                }
                ASTTypeKind::Component(component) => self.resolve_component_signature(component)?,
            };
            Ok((ty.owner, id))
        } else {
            Err(HIRError::type_unrecognized(real.name(), ty.span))
        }
    }
    ///Gets the signature of the given `f` function. Asserting the id of the file it was generated is the given `file`.
    fn get_signature_of_function(&self, f: &FuncDeclaration) -> Result<DedupPoolId<HirType>> {
        let ret = self.find_type(f.return_type)?.1;
        let args = f
            .args
            .iter()
            .map(|f| {
                let inner = f.data.kind;
                self.find_type(inner).map(|v| v.1)
            })
            .collect::<Result<_>>()?;
        Ok(self.hir.create_function_type(args, ret))
    }

    /// Pure computation of a component's signature type (no cycle detection).
    fn compute_component_type(
        &self,
        component: &ComponentDeclaration,
    ) -> Result<DedupPoolId<HirType>> {
        let name = self.type_name(component.name.data);
        let (properties, children) = {
            let mut properties = Vec::with_capacity(component.members.len());
            let mut components = Vec::with_capacity(component.members.len());
            for member in &component.members {
                match &member.kind {
                    ComponentMemberKind::Property { name, ty, .. } => {
                        if let Some(ty) = ty {
                            let (_, field) = self.find_type(*ty)?;
                            properties.push((*name, field));
                        } else {
                            return Err(HIRError::component_missing_prop_type(member.span));
                        }
                    }
                    ComponentMemberKind::Child(c) => {
                        let (_, ty) = self.find_type(c.data.name)?;
                        let view = self.hir.view(ty);
                        if let Some(view) = view.is_component() {
                            components.push(view.data);
                        } else {
                            let name = self.type_name(c.data.name.data);
                            return Err(HIRError::not_a_component(name, c.span));
                        };
                    }
                }
            }
            (properties, components)
        };
        Ok(self.hir.create_component_type(name, properties, children))
    }

    /// Resolve a component's signature with cycle detection.
    pub(crate) fn resolve_component_signature(
        &self,
        component: &ComponentDeclaration,
    ) -> Result<DedupPoolId<HirType>> {
        let name = self.type_name(component.name.data);
        let key = (self.entry, name);

        // Push onto cycle-detection stack
        self.pendings.signature_stack.borrow_mut().push(key);

        // Insert into in-progress set. If already present, we have a cycle.
        if !self.pendings.signatures_in_progress.insert(key) {
            let chain = self.pendings.signature_stack.borrow().clone();
            self.pendings.signature_stack.borrow_mut().pop();
            return Err(HIRError::cyclic_component_signature(
                name,
                chain,
                component.span,
            ));
        }

        let result = self.compute_component_type(component);

        self.pendings.signatures_in_progress.remove(&key);
        self.pendings.signature_stack.borrow_mut().pop();
        result
    }
}

impl<'a> HirQueueBuilder<'a> {
    pub fn new(hir: &'a SlynxHir<'a>, modules: &'a Modules<'a>) -> Self {
        Self {
            hir,
            modules,
            bodies: WorkChannel::new(),
            statics: WorkChannel::new(),
            components: WorkChannel::new(),
            resolved_bodies: DashMap::new(),
            resolved_components: DashMap::new(),
            bodies_in_progress: DashSet::new(),
            signature_stack: RefCell::new(Vec::new()),
            signatures_in_progress: DashSet::new(),
        }
    }

    pub(crate) fn close_bodies(&mut self) {
        self.bodies.close_sender();
    }

    pub(crate) fn get_node(&self, id: FileId) -> HirNode<'_> {
        HirNode {
            hir: self.hir,
            modules: self.modules,
            entry: id,
            pendings: PendingSignatures {
                signatures_in_progress: &self.signatures_in_progress,
                signature_stack: &self.signature_stack,
            },
        }
    }
    ///Hoists the given function, and then enqueues it so its body can be checked. On being processed, this function might generate more than simply the given `f` function since it will generate all the dependencies of `f` to work. Including impures
    pub(crate) fn enqueue_static(
        &self,
        s: &StaticDeclaration,
        node: HirNode<'_>,
    ) -> Result<DeclarationId<HirStaticDeclaration>> {
        let (_, ty) = node.find_type(s.ty)?;
        let name = s.name;
        let id = self.hir.symbols_registry.get_or_insert_static(
            HirSymbol::new(node.entry, name),
            || {
                let decl = HirStaticDeclaration {
                    name,
                    ty,
                    visibility: s.visibility,
                    external: s.external,
                    attributes: Vec::new(),
                };
                let file = self.hir.get_or_create_file(node.entry);
                file.create_static(decl)
            },
        );

        // Process attributes after the declaration is registered
        let decl_id = crate::id::AnyDeclarationId::new(
            node.entry,
            crate::id::AnyLocalDeclarationId::Static(id.local_id),
        );
        let attrs = attributes::process_attributes(self.hir, &s.attributes, decl_id);
        if !attrs.is_empty() {
            self.hir
                .get_file_mut(node.entry)
                .declarations
                .statik
                .get_mut(id.local_id)
                .attributes = attrs;
        }

        self.statics.send(());
        Ok(id)
    }

    pub(crate) fn process(self) -> Result<()> {
        loop {
            select! {
                recv(self.bodies.receiver()) -> body => {
                    if let Ok(PendantBody { func_id, body, argument_names }) = body {
                        let mut builder = HirFunctionBuilder::new(func_id);
                        for (idx, name) in argument_names.into_iter().enumerate() {
                            builder.create_argument(&self, name, idx as u8);
                        }
                        let ExpressionBuildResult { statements, args } = builder.build_body(&self, body)?;
                        self.resolved_bodies.insert(func_id, (statements, args));
                    }else {
                        break;
                    }
                }
                recv(self.components.receiver()) -> component => {
                    if let Ok(PendantComponent { owner, component }) = component {
                        let decls = self.component_body(owner, &self, component)?;
                        self.resolved_components.insert(owner, decls);
                    }
                }
            }
        }

        for mut entry in self.resolved_bodies.iter_mut() {
            let mut file = self.hir.get_file_mut(entry.key().file_id);
            let func = file.declarations.functions.get_mut(entry.key().local_id);
            func.statements.append(&mut entry.0);
            for data in entry.1.drain(..) {
                func.args.push(data);
            }
        }
        for mut entry in self.resolved_components.iter_mut() {
            let mut file = self.hir.get_file_mut(entry.key().file_id);
            let func = file.declarations.components.get_mut(entry.key().local_id);
            func.props.append(&mut entry);
        }
        Ok(())
    }
}

impl<'a> HirQueueBuilder<'a> {
    /// Lazily resolves a method on a struct type. Looks up the `ObjectDeclaration`
    /// from the AST, creates the function declaration, registers it as a method
    /// on the type, and enqueues the body for processing.
    pub(crate) fn resolve_method(
        &self,
        file_id: FileId,
        struct_ty: DedupPoolId<HirType>,
        method_name: SymbolPointer,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let struct_id = match self.hir.types_module[struct_ty] {
            HirType::Struct(id) => id,
            _ => return Ok(None),
        };
        let struct_name = self.hir.get_struct_name(struct_id);

        let ast_type = self.modules.find_type_inside_module(file_id, struct_name);
        let (obj_file_id, obj_decl) = match ast_type {
            Some(ASTType {
                owner,
                content: ASTTypeKind::Struct(decl),
            }) => (owner, decl),
            _ => return Ok(None),
        };

        let method = obj_decl
            .methods
            .iter()
            .find(|m| self.modules.type_name(m.method_name.data) == method_name);

        let Some(method) = method else {
            return Ok(None);
        };

        let self_sym = self.hir.intern_name("Self");

        let node = self.get_node(file_id);

        let mut args = Vec::with_capacity(method.arguments.len());
        for arg in &method.arguments {
            let ty = if self.modules.type_name(arg.data.kind.data) == self_sym {
                struct_ty
            } else {
                let (_, ty) = node.find_type(arg.data.kind)?;
                ty
            };
            args.push(ty);
        }

        let return_type = if self.modules.type_name(method.return_type.data) == self_sym {
            struct_ty
        } else {
            let (_, ty) = node.find_type(method.return_type)?;
            ty
        };

        let func_ty = self.hir.create_function_type(args, return_type);

        let mangled = format!(
            "{}_{}",
            self.hir.get_name(method_name),
            self.hir.get_name(struct_name),
        );
        let mangled_symbol = self.hir.intern_name(&mangled);

        let decl_id = self.hir.symbols_registry.get_or_insert_function(
            HirSymbol::new(obj_file_id, mangled_symbol),
            || {
                let decl = HirFunctionDeclaration {
                    name: method_name,
                    args: Default::default(),
                    ty: func_ty,
                    statements: Vec::new(),
                    visibility: obj_decl.visibility,
                    external: obj_decl.external,
                    attributes: Vec::new(),
                };
                let file = self.hir.get_or_create_file(obj_file_id);
                file.create_function(decl)
            },
        );

        self.hir
            .types_module
            .create_method(struct_ty, method_name, decl_id);

        if !obj_decl.external {
            let arg_names: Vec<SymbolPointer> =
                method.arguments.iter().map(|arg| arg.data.name).collect();
            self.bodies.send(PendantBody {
                func_id: decl_id,
                body: &method.body,
                argument_names: arg_names,
            });
        }

        Ok(Some(decl_id))
    }
}

impl<'a> Deref for HirQueueBuilder<'a> {
    type Target = Modules<'a>;
    fn deref(&self) -> &Self::Target {
        self.modules
    }
}
