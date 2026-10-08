pub(crate) mod attributes;
pub(crate) mod component;
mod expression;
mod function;
mod generic;
pub(crate) mod interfaces;
mod lowering;
mod work_channel;
use std::{cell::RefCell, ops::Deref};

use common::{
    Spanned,
    pool::{DedupPoolId, PoolId},
};

use crate::{
    ComponentId, ComponentMemberDeclaration, DeclarationId, HirComponentDeclaration,
    HirFunctionDeclaration, HirStatement, HirStaticDeclaration, Owned, Result, SlynxHir,
    SymbolPointer, VariableId,
    attributes::process_attributes,
    builders::{
        expression::ExpressionBuildResult, function::HirFunctionBuilder, lowering::ASTLowerer,
        work_channel::WorkChannel,
    },
    context::HirSymbol,
    term::TermId,
};
use crossbeam_channel::select;
use dashmap::{DashMap, DashSet};
pub use expression::*;
use module_loader::{FileId, Modules};
use slynx_parser::{
    ASTStatement, ComponentDeclaration, ExtendDeclaration, GenericIdentifier, StaticDeclaration,
    Type, TypeContext,
};

/// Orchestrates the AST → HIR build: hoists `main`, enqueues its transitive
/// dependencies, resolves bodies, and closes the work channels.
///
/// This is the entry point of the lowerer; it drives [`HirQueueBuilder`]
/// against an immutable `&SlynxHir` facade (which owns the mutable data).
pub(crate) fn generate_hir<'a>(hir: &'a SlynxHir<'a>, modules: &'a Modules<'a>) -> Result<()> {
    let builder = HirQueueBuilder::new(hir, modules);
    builder.validate_interface_syntax(modules.entries()[0].id)?;

    {
        let entry = &modules.entries()[0];
        let main_symbol = hir.intern_name("main");
        if let Some(mainfunc) = entry.func().iter().find(|func| func.name == main_symbol) {
            builder.enqueue_function(mainfunc, entry.id)?;
            builder.process()?;
        }
    }
    builder.close_bodies();
    Ok(())
}

pub struct PendingSignatures<'a> {
    /// Signature resolution state per component (by (FileId, SymbolPointer)).
    pub signatures_in_progress: &'a DashSet<(FileId, SymbolPointer)>,
    pub signature_stack: &'a RefCell<Vec<(FileId, SymbolPointer)>>,
}

///A Node represents a file that is being compiled on the HIR. It's just a view over the Hir and AST to properly read data from the ast from the `entry` file
pub struct HirNode<'a> {
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
    self_type: Option<TermId>,
}

pub(crate) struct PendantComponent<'a> {
    owner: DeclarationId<HirComponentDeclaration>,
    component: &'a ComponentDeclaration,
}

pub struct HirQueueBuilder<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    pub(crate) lowerer: ASTLowerer<'a>,
    pub(crate) interface_implementations: DashMap<
        (FileId, PoolId<ExtendDeclaration>, TermId),
        DeclarationId<crate::HirExtendDeclaration>,
    >,
    pub(crate) bodies: WorkChannel<PendantFunction<'a>>,
    pub(crate) statics: WorkChannel<()>,
    // TODO(interfaces): populated when the HIR extend scaffold is wired in.
    #[allow(dead_code)]
    pub(crate) interfaces: WorkChannel<()>,
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

impl HirNode<'_> {}

impl<'a> HirQueueBuilder<'a> {
    pub fn new(hir: &'a SlynxHir<'a>, modules: &'a Modules<'a>) -> Self {
        Self {
            hir,
            modules,
            lowerer: ASTLowerer::new(modules),
            interface_implementations: DashMap::new(),
            bodies: WorkChannel::new(),
            statics: WorkChannel::new(),
            components: WorkChannel::new(),
            interfaces: WorkChannel::new(),
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
        requester: FileId,
    ) -> Result<DeclarationId<HirStaticDeclaration>> {
        let ty = self
            .lowerer
            .lower_type(self, requester, s.ty, &TypeContext::EMPTY)?
            .term;
        let name = s.name;
        let id = self.hir.symbols_registry.get_or_insert_static(
            HirSymbol::new(requester, name),
            || {
                let file = self.hir.store.get_or_create_file(requester);
                let id = file.insert_at_statik_with_id(|id| {
                    let attributes =
                        process_attributes(self.hir, Owned::new(requester, id), &s.attributes)?;
                    Ok(HirStaticDeclaration {
                        name,
                        span: s.span,
                        ty,
                        visibility: s.visibility,
                        external: s.external,
                        attributes,
                    })
                })?;
                Ok(Owned::new(requester, id))
            },
        )?;

        self.statics.send(());
        Ok(id)
    }

    pub(crate) fn process(&self) -> Result<()> {
        loop {
            select! {
                recv(self.bodies.receiver()) -> body => {
                    let Ok(PendantFunction { func_id, body, argument_names, context, self_type }) = body else {
                        break;
                    };
                    let mut builder = HirFunctionBuilder::new(func_id, self_type);
                    for (idx, name) in argument_names.into_iter().enumerate() {
                        builder.create_argument(self, name, idx as u8);
                    }
                    let ExpressionBuildResult { statements, args } = builder.build_body(self, body, &context)?;
                    self.resolved_bodies.insert(func_id, (statements, args));
                    if self.bodies.receiver().is_empty() {
                        break;
                    }

                }
                recv(self.components.receiver()) -> component => {
                    if let Ok(PendantComponent { owner, component }) = component {
                        let decls = self.component_body(owner, self, component)?;
                        self.resolved_components.insert(owner, decls);
                    }
                }
            }
        }

        for mut entry in self.resolved_bodies.iter_mut() {
            let mut file = self.hir.get_file_mut(entry.key().owner);
            let func = file.declarations.functions.get_mut(entry.key().term);
            func.statements.append(&mut entry.0);
            for data in entry.1.drain(..) {
                func.args.push(data);
            }
        }
        for mut entry in self.resolved_components.iter_mut() {
            let mut file = self.hir.get_file_mut(entry.key().owner);
            let func = file.declarations.components.get_mut(entry.key().term);
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
