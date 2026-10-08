use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{
    ASTExpression, AliasDeclaration, EnumDeclaration, EnumVariantKind, FuncDeclaration,
    GenericsMetadata, InterfaceDeclaration, ObjectDeclaration, Type, TypeContext,
};

use crate::{
    EnumVariantType, GenericParameter, HIRError, HirEnumDeclaration, HirObjectDeclaration,
    HirQueueBuilder, Owned, Result, Visible,
    builders::lowering::ASTLowerer,
    error::MissingFeature,
    interface::InterfaceTerm,
    term::{PrimitiveType, Term, TermId, TermNode},
};

pub struct LowerTypeDeclarationDescriptor<'a> {
    pub ast_type: ASTType,
    pub context: &'a TypeContext<'a>,
    pub span: Span,
}

impl<'a> ASTLowerer<'a> {
    fn lower_interface_method_signature(
        &self,
        queue: &HirQueueBuilder<'a>,
        owner: FileId,
        method: &FuncDeclaration,
    ) -> Result<TermId> {
        let context = TypeContext::new(&method.generics.type_params);
        let self_symbol = queue.hir.intern_name("Self");
        let self_type = queue
            .hir
            .types
            .create_type(Term::new_variable_type(0, self_symbol));
        let lower = |ty: Spanned<DedupPoolId<Type>>| {
            self.lower_type_with_self(queue, owner, ty, &context, self_type)
                .map(|owned| owned.term)
        };
        let return_type = lower(method.return_type)?;
        let arguments = method
            .args
            .iter()
            .map(|argument| lower(argument.data.kind))
            .collect::<Result<Vec<_>>>()?;
        Ok(queue.hir.types.create_function_type(arguments, return_type))
    }

    fn lower_interface(
        &self,
        queue: &HirQueueBuilder<'a>,
        owner: FileId,
        interface: PoolId<InterfaceDeclaration>,
        context: &TypeContext,
        span: Span,
    ) -> Result<TermId> {
        let declaration = queue.modules.get_entry(owner).interfaces().get(interface);
        if !declaration.generics.type_params.is_empty() {
            return Err(HIRError::unimplemented(
                MissingFeature::GenericInterfaces,
                span,
            ));
        }
        if let Some(method) = declaration
            .methods
            .iter()
            .find(|method| !method.generics.type_params.is_empty())
        {
            return Err(HIRError::unimplemented(
                MissingFeature::GenericInterfaces,
                method.span,
            ));
        }
        let methods = declaration
            .methods
            .iter()
            .map(|method| {
                self.lower_interface_method_signature(queue, owner, method)
                    .map(|signature| (method.name, signature))
            })
            .collect::<Result<Vec<_>>>()?;
        let signatures = methods
            .iter()
            .map(|(_, signature)| *signature)
            .collect::<Vec<_>>();
        let super_interfaces = declaration
            .super_interfaces
            .iter()
            .map(|ty| {
                self.lower_type(queue, owner, *ty, context)
                    .map(|owned| owned.term)
            })
            .collect::<Result<Vec<_>>>()?;
        let interface_term =
            queue
                .hir
                .types
                .create_interface_type(declaration.name, methods, super_interfaces);
        let interface = queue
            .hir
            .view(interface_term)
            .is_extension()
            .and_then(|extension| {
                extension
                    .as_any()
                    .downcast_ref::<InterfaceTerm>()
                    .map(|term| term.raw)
            })
            .expect("an interface type is always backed by an InterfaceTerm");
        // Each declared method becomes a bodyless declaration so calls on a
        // bounded generic parameter have a target to resolve against.
        for (method, signature) in declaration.methods.iter().zip(&signatures) {
            queue.insert_interface_method_signature(
                owner,
                interface,
                declaration.name,
                method,
                *signature,
            )?;
        }
        Ok(interface_term)
    }

    fn lower_alias(
        &self,
        queue: &HirQueueBuilder<'a>,
        owner: FileId,
        alias: PoolId<AliasDeclaration>,
        context: &TypeContext,
    ) -> Result<Owned<TermId>> {
        let target = queue.modules.get_entry(owner).alias().get(alias).target;
        self.lower_type(queue, owner, target, context)
    }

    fn lower_struct(
        &self,
        queue: &HirQueueBuilder<'a>,
        owner: FileId,
        strukt: PoolId<ObjectDeclaration>,
    ) -> Result<TermId> {
        let declaration = queue.modules.get_entry(owner).object().get(strukt);
        let context = TypeContext::new(&declaration.generics.type_params);
        let fields = declaration
            .fields
            .iter()
            .map(|field| {
                self.lower_type(queue, owner, field.name.data.kind, &context)
                    .map(|ty| Visible::new(field.visibility, (field.name.data.name.data, ty.term)))
            })
            .collect::<Result<Vec<_>>>()?;

        let struct_type = queue
            .hir
            .types
            .create_struct_type(declaration.name, fields, Vec::new());
        queue
            .hir
            .store
            .get_or_create_file(owner)
            .create_object(HirObjectDeclaration {
                name: declaration.name,
                span: declaration.span,
                generics: self.generic_parameters_of(
                    queue,
                    &declaration.generics,
                    owner,
                    &context,
                )?,
                ty: struct_type,
                visibility: declaration.visibility,
                external: declaration.external,
                attributes: Vec::new(),
            });
        Ok(struct_type)
    }

    fn lower_enum(
        &self,
        queue: &HirQueueBuilder<'a>,
        owner: FileId,
        enumer: PoolId<EnumDeclaration>,
    ) -> Result<TermId> {
        let declaration = queue.modules.get_entry(owner).enums().get(enumer);
        let context = TypeContext::new(&declaration.generics.type_params);
        if let Some(representation) = &declaration.representation {
            let representation_type = self
                .lower_type(
                    queue,
                    owner,
                    representation.span.make_spanned(representation.data),
                    &TypeContext::new(&[]),
                )?
                .term;
            match queue
                .hir
                .view(representation_type)
                .dereference()
                .raw()
                .node()
            {
                TermNode::Primitive(
                    PrimitiveType::Signed { .. } | PrimitiveType::Unsigned { .. },
                ) => {}
                _ => {
                    return Err(HIRError::invalid_enum_representation(
                        declaration.name,
                        representation.span,
                    ));
                }
            }
        }

        let mut next_discriminant: i32 = 0;
        let mut variants = Vec::with_capacity(declaration.variants.len());
        for variant in &declaration.variants {
            let (discriminant, payload) = match &variant.kind {
                EnumVariantKind::Raw => (next_discriminant, Vec::new()),
                EnumVariantKind::RawValued(rhs) => {
                    let value = match queue.modules.get_expr(rhs.data) {
                        ASTExpression::IntLiteral(value) => *value,
                        _ => {
                            return Err(HIRError::enum_variant_must_be_an_int(
                                variant.name.data,
                                variant.span,
                            ));
                        }
                    };
                    (value, Vec::new())
                }
                EnumVariantKind::Associated(types) => (
                    next_discriminant,
                    types
                        .iter()
                        .map(|ty| {
                            self.lower_type(queue, owner, *ty, &context)
                                .map(|owned| owned.term)
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
                EnumVariantKind::Struct(fields) => (
                    next_discriminant,
                    fields
                        .iter()
                        .map(|field| {
                            self.lower_type(queue, owner, field.data.kind, &context)
                                .map(|owned| owned.term)
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
            };
            next_discriminant = next_discriminant.max(discriminant.saturating_add(1));
            variants.push(EnumVariantType {
                name: variant.name.data,
                payload,
                discriminant,
            });
        }

        let enum_type = queue.hir.types.create_enum_type(declaration.name, variants);
        queue
            .hir
            .store
            .get_or_create_file(owner)
            .create_enum(HirEnumDeclaration {
                name: declaration.name,
                span: declaration.span,
                generics: self.generic_parameters_of(
                    queue,
                    &declaration.generics,
                    owner,
                    &context,
                )?,
                variants: Vec::new(),
                visibility: declaration.visibility,
                attributes: Vec::new(),
                ty: enum_type,
            });
        Ok(enum_type)
    }

    ///Materialized some AST declaration into the HIR, thus, inserting content on the HIR via the given `queue` and returns the ID of the created content
    fn lower_ast_declaration(
        &self,
        queue: &HirQueueBuilder<'a>,
        ast_type: ASTType,
        context: &TypeContext,
        span: Span,
    ) -> Result<Owned<TermId>> {
        let term = match ast_type.content {
            ASTTypeKind::Interface(interface) => {
                self.lower_interface(queue, ast_type.owner, interface, context, span)?
            }
            ASTTypeKind::Builtin(builtin) => queue.hir.types.create_type(builtin.into()),
            ASTTypeKind::Alias(alias) => {
                return self.lower_alias(queue, ast_type.owner, alias, context);
            }
            ASTTypeKind::Struct(strukt) => self.lower_struct(queue, ast_type.owner, strukt)?,
            ASTTypeKind::Component(component) => {
                let component = queue
                    .modules
                    .get_entry(ast_type.owner)
                    .component()
                    .get(component);
                self.resolve_component_signature(queue, &queue.get_node(ast_type.owner), component)?
            }
            ASTTypeKind::Enum(enumer) => self.lower_enum(queue, ast_type.owner, enumer)?,
        };
        Ok(Owned {
            owner: ast_type.owner,
            term,
        })
    }

    pub fn generic_parameters_of(
        &self,
        queue: &HirQueueBuilder<'a>,
        method: &GenericsMetadata,
        entry: FileId,
        context: &TypeContext,
    ) -> Result<Vec<GenericParameter>> {
        let clauses = method.clauses();
        let generics = {
            let mut out = Vec::new();
            for param in method.type_params() {
                if let Some(clause) = clauses.iter().find_map(|clause| {
                    if {
                        let ty = queue.modules.get_type(clause.type_to_check.data); //no get plain type because the error message would lead to incorrect message
                        !matches!(ty, Type::Plain(_))
                    } {
                        return Some(Err(HIRError::unimplemented(
                            MissingFeature::ComplexTypeForBounds,
                            clause.type_to_check.span,
                        )));
                    }
                    (queue.get_plain_type(clause.type_to_check).identifier == *param)
                        .then_some(Ok(clause))
                }) {
                    let clause = clause?;
                    let bounds = clause
                        .bounds
                        .iter()
                        .map(|bound| {
                            let ty = self.lower_type(queue, entry, *bound, &context)?;
                            if let Some(ext) = queue.hir.view(ty.term).is_extension()
                                && let Some(interface) =
                                    ext.as_any().downcast_ref::<InterfaceTerm>()
                            {
                                Ok(interface.clone())
                            } else {
                                Err(HIRError::expected_interface_type(ty.term, bound.span))
                            }
                        })
                        .collect::<Result<Vec<_>>>()?;

                    out.push(GenericParameter {
                        name: *param,
                        bounds,
                    });
                } else {
                    out.push(GenericParameter {
                        name: *param,
                        bounds: Vec::new(),
                    });
                };
            }
            out
        };
        Ok(generics)
    }
    ///Lowers a type declaration. This is a declaration that defines a type. If the given `descriptor.ast_type` is already lowered, it is returned from the cache instead of lowering it again.
    ///Lowering phase means that the content will be inserted into the HIR via queue if it does not exist, returning its ID. So if this is a struct, then it inserts an Struct declaration on the HIR and returns the ID of it
    pub fn materialize_type_declaration(
        &self,
        queue: &HirQueueBuilder<'a>,
        descriptor: LowerTypeDeclarationDescriptor,
    ) -> Result<Owned<TermId>> {
        if matches!(descriptor.ast_type.content, ASTTypeKind::Alias(_)) {
            return self.lower_ast_declaration(
                queue,
                descriptor.ast_type,
                descriptor.context,
                descriptor.span,
            );
        }
        if let Some(lowered) = self.lowered_types.get(&descriptor.ast_type) {
            return Ok(lowered.value().clone());
        }
        let lowered = self.lower_ast_declaration(
            queue,
            descriptor.ast_type,
            descriptor.context,
            descriptor.span,
        )?;

        self.lowered_types
            .insert(descriptor.ast_type, lowered.clone());

        Ok(lowered)
    }

    pub(crate) fn materialize_interface_definition(
        &self,
        queue: &HirQueueBuilder<'a>,
        ast_type: ASTType,
        span: Span,
    ) -> Result<Owned<TermId>> {
        if let Some(lowered) = self.lowered_types.get(&ast_type) {
            return Ok(lowered.value().clone());
        }
        let lowered = self.lower_ast_declaration(queue, ast_type, &TypeContext::EMPTY, span)?;
        self.lowered_types.insert(ast_type, lowered.clone());
        Ok(lowered)
    }
}
