use common::{Span, Spanned, pool::DedupPoolId};
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{Type, TypeContext};

use crate::{
    DeclarationId, DescriptorId, HIRError, HirFunctionDeclaration, HirNode, HirQueueBuilder,
    PendantFunction, Result, SymbolPointer,
    context::HirSymbol,
    term::{TermId, TermNode},
};

impl<'a> HirNode<'a> {
    ///Finds the 'Self' type of a struct based on the 'ty'. In case this is just a copy/paste of the given `ty` that will replace every occurrence of 'Self' to the given `selfty`. If `ty` is simply 'A', then it just returns 'selfty', if its &A, then '&selfty', and so on.
    pub fn find_self_type(&self, ty: DedupPoolId<Type>, selfty: TermId) -> TermId {
        // Delegates to the single shared Type → HIR-type walker (`HirNode::find_type_inner`)
        // with `Self` substituted, so wrapper types like `&Self` fork from
        // `find_type` instead of re-implementing the traversal.
        self.find_type_inner(
            Span::default().make_spanned(ty),
            &TypeContext::EMPTY,
            Some(selfty),
        )
        .map(|(_, ty)| ty)
        .expect("a type with Self substituted inside cannot fail to lower")
    }
}

impl<'a> HirQueueBuilder<'a> {
    /// Lazily resolves a method on a struct type. Looks up the `ObjectDeclaration`
    /// from the AST, creates the function declaration, registers it as a method
    /// on the type, and enqueues the body for processing.
    /// Returns the `DeclarationId` of the function declaration, if one was created, otherwise returns Ok(None).
    pub(crate) fn resolve_method(
        &self,
        file_id: FileId,
        struct_ty: TermId,
        method_name: SymbolPointer,
        span: Span,
    ) -> Result<Option<DeclarationId<HirFunctionDeclaration>>> {
        let struct_id = match self.hir.types[struct_ty].node() {
            TermNode::Data(descriptor) if let DescriptorId::Struct(id) = descriptor => *id,
            _ => return Err(HIRError::not_a_struct(struct_ty, span)),
        };
        let struct_name = self.hir.types.get_struct_name(struct_id);

        let ast_type = self.modules.find_type(file_id, struct_name);
        let (obj_file_id, obj_decl) = match ast_type {
            Some(ASTType {
                owner,
                content: ASTTypeKind::Struct(decl),
            }) => {
                let obj_decl = self.modules.get_entry(owner).object().get(decl);
                (owner, obj_decl)
            }
            _ => return Err(HIRError::not_a_struct(struct_ty, span)),
        };

        let method = obj_decl
            .methods
            .iter()
            .find(|m| m.method_name == method_name)
            .ok_or(HIRError::method_not_found(method_name, span))?;

        let self_sym = self.hir.intern_name("Self");
        let node = self.get_node(file_id);

        let context = TypeContext::new(&method.type_params);

        let find_self_type = |ty: Spanned<DedupPoolId<Type>>| {
            if let Some(name) = self.modules.referenced_name(ty.data)
                && name == self_sym
            {
                Ok(node.find_self_type(ty.data, struct_ty))
            } else {
                node.find_type(ty, &context).map(|v| v.1)
            }
        };
        let args = method
            .arguments
            .iter()
            .map(|arg| find_self_type(arg.data.kind))
            .collect::<Result<Vec<_>>>()?;
        let return_type = find_self_type(method.return_type)?;

        let func_ty = self.hir.types.create_function_type(args, return_type);

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
                    name: mangled_symbol,
                    generics: method.type_params.clone(),
                    args: Default::default(),
                    ty: func_ty,
                    statements: Vec::new(),
                    visibility: obj_decl.visibility,
                    external: obj_decl.external,
                    attributes: Vec::new(),
                    span: method.span,
                };
                let file = self.hir.store.get_or_create_file(obj_file_id);
                file.create_function(decl)
            },
        );

        self.hir
            .types
            .create_method(struct_ty, method_name, decl_id);

        if !obj_decl.external {
            let arg_names: Vec<SymbolPointer> = method
                .arguments
                .iter()
                .map(|arg| arg.data.name.data)
                .collect();
            self.bodies.send(PendantFunction {
                context: TypeContext::new(&method.type_params),
                func_id: decl_id,
                body: &method.body,
                argument_names: arg_names,
                self_type: Some(struct_ty),
            });
        }

        Ok(Some(decl_id))
    }
}
