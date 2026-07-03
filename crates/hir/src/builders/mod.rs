mod function;
mod work_channel;
use std::ops::Deref;

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use dashmap::DashMap;
use module_loader::{ASTType, ASTTypeKind, FileId, Modules, SourceNode};
use slynx_parser::{
    ASTStatement, ComponentMemberKind, FuncDeclaration, GenericIdentifier, StaticDeclaration,
};

use crate::{
    DeclarationId, HIRError, HirFunctionDeclaration, HirObjectDeclaration, HirStatement,
    HirStaticDeclaration, HirType, Result, SlynxHir, SymbolPointer, VariableId,
    builders::{
        function::{HirFunctionBuildResult, HirFunctionBuilder},
        work_channel::WorkChannel,
    },
    context::HirSymbol,
    error::InvalidTypeReason,
};

///A Node represents a file that is being compiled on the HIR. It's just a view over the Hir and AST to properly read data from the ast from the `entry` file
pub struct HirNode<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    ///The ID of the file that we are reading
    pub(crate) entry: FileId,
}

impl<'a> Deref for HirNode<'a> {
    type Target = Modules<'a>;
    fn deref(&self) -> &Self::Target {
        self.modules
    }
}

struct PendantBody<'a> {
    func_id: DeclarationId<HirFunctionDeclaration>,
    body: &'a [Spanned<DedupPoolId<ASTStatement>>],
    argument_names: Vec<SymbolPointer>,
}

pub struct HirQueueBuilder<'a> {
    pub(crate) hir: &'a SlynxHir<'a>,
    pub(crate) modules: &'a Modules<'a>,
    pub(crate) bodies: WorkChannel<PendantBody<'a>>,
    pub(crate) statics: WorkChannel<()>,
    pub(crate) resolved_bodies: DashMap<
        DeclarationId<HirFunctionDeclaration>,
        (Vec<Spanned<PoolId<HirStatement>>>, Vec<VariableId>),
    >,
}

impl HirNode<'_> {
    pub fn get_source_node(&self) -> &SourceNode {
        self.modules.get_entry(self.entry)
    }
    ///Finds the Hir type for the given `ty` and what file contains it if theres some. The given `file` is the file id where the given `ty` was generated at
    fn find_type(
        &self,
        ty: Spanned<DedupPoolId<GenericIdentifier>>,
    ) -> Result<(FileId, DedupPoolId<HirType>)> {
        let real = self.modules.get_type(ty.data);
        if let Some(ty) = self.modules.find_type_inside_module(
            &self.modules.entries()[self.entry.as_raw() as usize],
            real.identifier,
        ) {
            let id = match ty.content {
                ASTTypeKind::Builtin(builtin) => self.hir.create_type(builtin.into()),
                ASTTypeKind::Alias(alias) => {
                    return self.find_type(alias.target);
                }
                ASTTypeKind::Struct(s) => {
                    let struct_name = self.get_type(s.name.data).identifier;
                    let fields = s
                        .fields
                        .iter()
                        .map(|field| {
                            let field_name = field.name.data.name;
                            let field_ty = field.name.data.kind;
                            let (_, type_id) = self.find_type(field_ty)?;
                            Ok((field_name, type_id))
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
                        });
                    }
                    struct_ty
                }
                ASTTypeKind::Component(component) => {
                    let mut properties = Vec::new();
                    for member in &component.members {
                        if let ComponentMemberKind::Property { name, ty, .. } = &member.kind {
                            let ty_id = self
                                .find_type(ty.ok_or_else(|| {
                                    let name = self.get_type(component.name.data).identifier;
                                    HIRError::invalid_type(
                                        name,
                                        InvalidTypeReason::CouldntInfer,
                                        member.span,
                                    )
                                })?)
                                .map(|(_, t)| t)?;
                            properties.push((*name, ty_id));
                        }
                    }
                    let name_id = self.get_type(component.name.data).identifier;
                    self.hir
                        .create_component_type(name_id, properties, Vec::new())
                }
            };
            Ok((ty.owner, id))
        } else {
            Err(HIRError::type_unrecognized(real.identifier, ty.span))
        }
    }
    ///Gets the signature of the given `f` function. Asserting the id of the file it was generated is the given `file`.
    fn get_signature_of(&self, f: &FuncDeclaration) -> Result<DedupPoolId<HirType>> {
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
}

impl<'a> HirQueueBuilder<'a> {
    pub fn new(hir: &'a SlynxHir<'a>, modules: &'a Modules<'a>) -> Self {
        Self {
            hir,
            modules,
            bodies: WorkChannel::new(),
            statics: WorkChannel::new(),
            resolved_bodies: DashMap::new(),
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
                };
                let file = self.hir.get_or_create_file(node.entry);
                file.create_static(decl)
            },
        );

        self.statics.send(());
        Ok(id)
    }
    ///Hoists the given function, and then enqueues it so its body can be checked. On being processed, this function might generate more than simply the given `f` function since it will generate all the dependencies of `f` to work. Including impures
    pub(crate) fn enqueue_function(
        &self,
        f: &'a FuncDeclaration,
        node: HirNode<'_>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let name = self.modules.get_type(f.name.data).identifier;
        let signature = node.get_signature_of(f)?;
        let names = f.args.iter().map(|arg| arg.data.name).collect();
        let id = self.hir.symbols_registry.get_or_insert_function(
            HirSymbol::new(node.entry, name),
            || {
                let decl = HirFunctionDeclaration {
                    name,
                    args: Default::default(),
                    ty: signature,
                    statements: Vec::new(),
                    visibility: f.visibility,
                    external: f.external,
                };
                let file = self.hir.get_or_create_file(node.entry);
                file.create_function(decl)
            },
        );

        self.bodies.send(PendantBody {
            func_id: id,
            body: &f.body,
            argument_names: names,
        });
        Ok(id)
    }

    ///Finds a function with the given `name` and returns it's id. If not found on the `requester` it tries to find on other files the requester imports. If not recognized by any, then hoists it properly
    pub fn find_function_named(
        &self,
        name: SymbolPointer,
        requester: &'a HirNode,
        span: Span,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        if let Some(func) = self
            .hir
            .find_function_by_symbol(HirSymbol::new(requester.entry, name))
        {
            Ok(func)
        } else if let Some(func) = self
            .hir
            .get_file(requester.entry)
            .find_function_with_name(name)
        {
            Ok(func)
        } else if let Some((id, func)) =
            requester.find_function_declaration(name, requester.get_source_node())
        {
            self.enqueue_function(func, self.get_node(id))
        } else {
            Err(HIRError::name_unrecognized(name, span))
        }
    }
    /// Lazily resolves a method on a struct type. Looks up the `ObjectDeclaration`
    /// from the AST, creates the function declaration, registers it as a method
    /// on the type, and enqueues the body for processing.
    pub(crate) fn resolve_method(
        &self,
        file_id: FileId,
        struct_ty: DedupPoolId<HirType>,
        method_name: SymbolPointer,
        _span: Span,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let struct_id = match self.hir.types_module[struct_ty] {
            HirType::Struct(id) => id,
            _ => return Ok(None),
        };
        let struct_name = self.hir.get_struct_name(struct_id);

        let source_node = self.modules.get_entry(file_id);

        let ast_type = self
            .modules
            .find_type_inside_module(source_node, struct_name);
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
            .find(|m| self.modules.get_type(m.method_name.data).identifier == method_name);

        let Some(method) = method else {
            return Ok(None);
        };

        let self_sym = self.hir.intern_name("Self");

        let node = self.get_node(file_id);

        let mut args = Vec::with_capacity(method.arguments.len());
        for arg in &method.arguments {
            let ty = if self.modules.get_type(arg.data.kind.data).identifier == self_sym {
                struct_ty
            } else {
                let (_, ty) = node.find_type(arg.data.kind)?;
                ty
            };
            args.push(ty);
        }

        let return_type = if self.modules.get_type(method.return_type.data).identifier == self_sym {
            struct_ty
        } else {
            let (_, ty) = node.find_type(method.return_type)?;
            ty
        };

        drop(node);

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
    pub(crate) fn process(&self) -> Result<()> {
        while let Some(PendantBody {
            func_id,
            body,
            argument_names,
        }) = self.bodies.recv()
        {
            let mut builder = HirFunctionBuilder::new(func_id);
            for (idx, name) in argument_names.into_iter().enumerate() {
                builder.create_argument(self, name, idx as u8);
            }
            let HirFunctionBuildResult { statements, args } = builder.build_body(self, &body)?;
            self.resolved_bodies.insert(func_id, (statements, args));
        }
        for mut entry in self.resolved_bodies.iter_mut() {
            let mut file = self.hir.get_file_mut(entry.key().file_id);
            let func = file.declarations.functions.get_mut(entry.key().local_id);
            func.statements.append(&mut entry.0);
            for data in entry.1.drain(..) {
                func.args.push(data);
            }
        }
        Ok(())
    }
}
impl<'a> Deref for HirQueueBuilder<'a> {
    type Target = Modules<'a>;
    fn deref(&self) -> &Self::Target {
        &self.modules
    }
}
