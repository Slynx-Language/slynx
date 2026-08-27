pub(crate) mod attributes;
pub(crate) mod component;
mod expression;
mod function;
mod structs;
pub(crate) mod styles;
mod work_channel;
use std::{cell::RefCell, collections::VecDeque, ops::Deref};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
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
use crossbeam_channel::select;
use dashmap::{DashMap, DashSet};
pub use expression::*;
use module_loader::{ASTTypeKind, FileId, Modules};
use slynx_parser::{
    ASTExpression, ASTStatement, ComponentDeclaration, ComponentMemberKind, FuncDeclaration,
    GenericIdentifier, StaticDeclaration, Type, TypeContext,
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

pub(crate) struct PendantFunction<'a> {
    func_id: DeclarationId<HirFunctionDeclaration>,
    context: TypeContext<'a>,
    body: &'a [Spanned<DedupPoolId<ASTStatement>>],
    argument_names: Vec<SymbolPointer>,
    self_type: Option<DedupPoolId<HirType>>,
}

pub(crate) struct PendantComponent<'a> {
    owner: DeclarationId<HirComponentDeclaration>,
    component: &'a ComponentDeclaration,
}

pub struct HirQueueBuilder<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    pub(crate) bodies: WorkChannel<PendantFunction<'a>>,
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
    ///Tries to find a type with the given `name`. For example, if the given name is `Person` it will try to find a type named like so, which might be a builtin type, an alias type, a struct, etc,
    ///something that is a type, and contains the given `name`
    pub fn find_type_named_as(
        &self,
        name: Spanned<SymbolPointer>,
        context: &TypeContext,
    ) -> Result<(FileId, DedupPoolId<HirType>)> {
        if let Some(data) = self.modules.find_type_inside_module(self.entry, name.data) {
            let id = match data.content {
                ASTTypeKind::Builtin(builtin) => self.hir.create_type(builtin.into()),
                ASTTypeKind::Alias(alias) => {
                    return self.find_type(alias.target, context);
                }
                ASTTypeKind::Struct(s) => {
                    let struct_name = s.name;
                    // Fields are typed against the object's own type
                    // parameters, not the referencing scope's, so a template
                    // like `object Option<T> { value: T }` keeps its `T`
                    // regardless of where `Option<int>` appears.
                    let struct_context = TypeContext::new(&s.type_params);
                    let fields = s
                        .fields
                        .iter()
                        .map(|field| {
                            let field_name = field.name.data.name;
                            let field_ty = field.name.data.kind;
                            let (_, type_id) = self.find_type(field_ty, &struct_context)?;

                            Ok(Visible::new(field.visibility, (field_name.data, type_id)))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let struct_ty = self.hir.create_struct_type(struct_name, fields, Vec::new());
                    // Register a HirObjectDeclaration so the codegen's
                    // hoist_declarations can create an IR struct for this type.
                    let file = self.hir.get_or_create_file(data.owner);
                    let already = file
                        .declarations
                        .objects
                        .iter()
                        .any(|d| d.name == struct_name);
                    if !already {
                        file.create_object(HirObjectDeclaration {
                            name: struct_name,
                            generics: s.type_params.clone(),
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
            Ok((data.owner, id))
        } else {
            Err(HIRError::type_unrecognized(name.data, name.span))
        }
    }

    ///Finds the Hir type for the given `ty` and what file contains it if theres some. The given `file` is the file id where the given `ty` was generated at
    pub fn find_type(
        &self,
        ty: Spanned<DedupPoolId<Type>>,
        context: &TypeContext,
    ) -> Result<(FileId, DedupPoolId<HirType>)> {
        let real = self.modules.get_type(ty.data);
        match real {
            Type::Plain(generic) => {
                let (owner, ty) =
                    self.find_type_named_as(ty.span.make_spanned(generic.identifier), context)?;
                if generic.generic.is_empty() {
                    return Ok((owner, ty));
                }
                // A generic application like `Option<int>` or `List<int>` is
                // represented as a Reference carrying the concrete type
                // arguments, so monomorphization can specialize it later.
                let ty_view = self.hir.view(ty);
                let deref = ty_view.dereference();
                if deref.is_struct().is_none() && deref.is_component().is_none() {
                    return Ok((owner, ty));
                }
                let args = generic
                    .generic
                    .iter()
                    .map(|arg| {
                        self.find_type(arg.span.make_spanned(arg.data), context)
                            .map(|v| v.1)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok((
                    owner,
                    self.hir.create_type(HirType::new_generic_ref(ty, args)),
                ))
            }
            Type::Array(t, len) => {
                let (id, ty) = self.find_type(ty.span.make_spanned(*t), context)?;
                let len = match self.modules.get_expr(*len) {
                    ASTExpression::IntLiteral(i) => *i as usize,
                    _ => unimplemented!(
                        "Array length can only be used as integers at the moment. It is idealized to be used in comptime in the future"
                    ),
                };
                let ty = self.hir.create_type(HirType::Array(ty, len));
                Ok((id, ty))
            }
            Type::Vector(t) => {
                let (id, ty) = self.find_type(ty.span.make_spanned(*t), context)?;
                let ty = self.hir.create_type(HirType::Vector(ty));
                Ok((id, ty))
            }
            Type::Reference(t) => {
                let (id, ty) = self.find_type(ty.span.make_spanned(*t), context)?;
                let ty = self.hir.create_type(HirType::ImutableRef(ty));
                Ok((id, ty))
            }
            Type::MutableReference(t) => {
                let (id, ty) = self.find_type(ty.span.make_spanned(*t), context)?;
                let ty = self.hir.create_type(HirType::MutableRef(ty));
                Ok((id, ty))
            }

            Type::Nullable(nullable) => {
                let (id, ty) = self.find_type(ty.span.make_spanned(*nullable), context)?;
                let ty = self.hir.create_type(HirType::Nullable(ty));
                Ok((id, ty))
            }
            Type::Generic(index) => {
                let ty = self.hir.create_type(HirType::GenericParam {
                    index: *index,
                    name: context.generic_names[*index as usize],
                });
                Ok((self.entry, ty))
            }
        }
    }
    ///Gets the signature of the given `f` function. Asserting the id of the file it was generated is the given `file`.
    fn get_signature_of_function(&self, f: &FuncDeclaration) -> Result<DedupPoolId<HirType>> {
        let context = TypeContext::new(&f.type_params);
        let ret = self.find_type(f.return_type, &context)?.1;
        let args = f
            .args
            .iter()
            .map(|f| {
                let inner = f.data.kind;
                self.find_type(inner, &context).map(|v| v.1)
            })
            .collect::<Result<_>>()?;
        Ok(self.hir.create_function_type(args, ret))
    }

    ///Resolves the explicit generic type arguments of a call like
    ///`compare<int>(a, b)` into their HIR type ids. Types that are generic
    ///parameters of the enclosing declaration (e.g. `identity<T>(x)`) resolve
    ///to [`HirType::GenericParam`] ids, which monomorphization later
    ///substitutes with concrete types.
    pub fn resolve_call_generics(
        &self,
        generics: &[Spanned<DedupPoolId<Type>>],
        context: &TypeContext,
    ) -> Result<Vec<DedupPoolId<HirType>>> {
        generics
            .iter()
            .map(|ty| self.find_type(*ty, context).map(|(_, ty)| ty))
            .collect()
    }

    /// Pure computation of a component's signature type (no cycle detection).
    fn compute_component_type(
        &self,
        component: &ComponentDeclaration,
    ) -> Result<DedupPoolId<HirType>> {
        let context = TypeContext::new(&component.type_params);
        let (properties, children) = {
            let mut properties = Vec::with_capacity(component.members.len());
            let mut components = Vec::with_capacity(component.members.len());
            for member in &component.members {
                match &member.kind {
                    ComponentMemberKind::Property { name, ty, .. } => {
                        if let Some(ty) = ty {
                            let (_, field) = self.find_type(*ty, &context)?;
                            properties.push((*name, field));
                        } else {
                            return Err(HIRError::component_missing_prop_type(member.span));
                        }
                    }
                    ComponentMemberKind::Child(c) => {
                        let (_, ty) = self.find_type(c.data.name, &context)?;
                        let ty_view = self.hir.view(ty);
                        let view = ty_view.dereference();
                        if let Some(view) = view.is_component() {
                            components.push(view.data);
                        } else {
                            let name = self.type_name(c.data.name.data, &TypeContext::new(&[]));
                            return Err(HIRError::not_a_component(name, c.span));
                        };
                    }
                }
            }
            (properties, components)
        };
        Ok(self
            .hir
            .create_component_type(component.name, properties, children))
    }

    /// Resolve a component's signature with cycle detection.
    pub(crate) fn resolve_component_signature(
        &self,
        component: &ComponentDeclaration,
    ) -> Result<DedupPoolId<HirType>> {
        let key = (self.entry, component.name);

        // Push onto cycle-detection stack
        self.pendings.signature_stack.borrow_mut().push(key);

        // Insert into in-progress set. If already present, we have a cycle.
        if !self.pendings.signatures_in_progress.insert(key) {
            let chain = self.pendings.signature_stack.borrow().clone();
            self.pendings.signature_stack.borrow_mut().pop();
            return Err(HIRError::cyclic_component_signature(
                component.name,
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
    pub fn get_plain_type(&self, ty: Spanned<DedupPoolId<Type>>) -> &GenericIdentifier {
        match self.modules.get_type(ty.data) {
            Type::Plain(generic) => generic,
            _ => panic!(
                "This function should only be called when the type of something is 100% true to be plain type"
            ),
        }
    }
    pub(crate) fn close_bodies(mut self) {
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
        let (_, ty) = node.find_type(s.ty, &TypeContext::EMPTY)?;
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

    pub(crate) fn process(&self) -> Result<()> {
        loop {
            select! {
                recv(self.bodies.receiver()) -> body => {
                    if let Ok(PendantFunction { func_id, body, argument_names, context, self_type }) = body {
                        let mut builder = HirFunctionBuilder::new(func_id, self_type);
                        for (idx, name) in argument_names.into_iter().enumerate() {
                            builder.create_argument(&self, name, idx as u8);
                        }
                        let ExpressionBuildResult { statements, args } = builder.build_body(&self, body, &context)?;
                        self.resolved_bodies.insert(func_id, (statements, args));

                        if self.bodies.receiver().is_empty() {
                            break;
                        }
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

impl<'a> Deref for HirQueueBuilder<'a> {
    type Target = Modules<'a>;
    fn deref(&self) -> &Self::Target {
        self.modules
    }
}
