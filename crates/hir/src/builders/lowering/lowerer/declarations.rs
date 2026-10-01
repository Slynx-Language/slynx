use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{
    ASTExpression, AliasDeclaration, EnumDeclaration, EnumVariantKind, FuncDeclaration,
    InterfaceDeclaration, ObjectDeclaration, Type, TypeContext,
};

use crate::{
    EnumVariantType, HIRError, HirEnumDeclaration, HirObjectDeclaration, HirQueueBuilder, Owned,
    Result, Visible,
    builders::lowering::ASTLowerer,
    error::InvalidTypeReason,
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
        let context = TypeContext::new(&method.type_params);
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
        if !declaration.type_args.is_empty() {
            return Err(HIRError::invalid_type(
                declaration.name,
                InvalidTypeReason::IncorrectUsage,
                span,
            ));
        }
        if let Some(method) = declaration
            .methods
            .iter()
            .find(|method| !method.type_params.is_empty())
        {
            return Err(HIRError::invalid_type(
                method.name,
                InvalidTypeReason::IncorrectUsage,
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
        let super_interfaces = declaration
            .super_interfaces
            .iter()
            .map(|ty| {
                self.lower_type(queue, owner, *ty, context)
                    .map(|owned| owned.term)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(queue
            .hir
            .types
            .create_interface_type(declaration.name, methods, super_interfaces))
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
        let context = TypeContext::new(&declaration.type_params);
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
                generics: declaration.type_params.clone(),
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
        let context = TypeContext::new(&declaration.type_params);
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
                generics: declaration.type_params.clone(),
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
    ///Lowers a type declaration. This is a declaration that defines a type. If the given `descriptor.ast_type` is already lowered, it is returned from the cache instead of lowering it again.
    ///Lowering phase means that the content will be inserted into the HIR via queue if it does not exist, returning its ID. So if this is a struct, then it inserts an Struct declaration on the HIR and returns the ID of it
    pub fn materialize_type_declaration(
        &self,
        queue: &HirQueueBuilder<'a>,
        descriptor: LowerTypeDeclarationDescriptor,
    ) -> Result<Owned<TermId>> {
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
}
