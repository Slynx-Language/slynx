use common::pool::DedupPoolId;
use module_loader::{ASTType, ASTTypeKind, FileId};
use slynx_parser::{ASTExpression, Type, TypeContext};

use crate::{
    DeclarationId, HirFunctionDeclaration, HirQueueBuilder, HirType, PendantFunction, Result,
    SymbolPointer, context::HirSymbol,
};

impl<'a> HirQueueBuilder<'a> {
    ///Finds the 'Self' type of a struct based on the 'ty'. In case this is just a copy/paste of the given `ty` that will replace every occurrence of 'Self' to the given `selfty`. If `ty` is simply 'A', then it just returns 'selfty', if its &A, then '&selfty', and so on.
    pub fn find_self_type(
        &self,
        ty: DedupPoolId<Type>,
        selfty: DedupPoolId<HirType>,
    ) -> DedupPoolId<HirType> {
        match self.modules.get_type(ty) {
            Type::Array(typ, len) => {
                let selftype = self.find_self_type(*typ, selfty);
                let len = match self.modules.get_expr(*len) {
                    ASTExpression::IntLiteral(i) => *i as usize,
                    _ => unimplemented!(
                        "Array length can only be used as integers at the moment. It is idealized to be used in comptime in the future"
                    ),
                };
                self.hir.create_type(HirType::Array(selftype, len))
            }
            Type::Vector(typ) => {
                let selftype = self.find_self_type(*typ, selfty);
                self.hir.create_type(HirType::Vector(selftype))
            }
            Type::Plain(_) => selfty,
            Type::Reference(inner) => {
                let selftype = self.find_self_type(*inner, selfty);
                self.hir.create_type(HirType::ImutableRef(selftype))
            }
            Type::MutableReference(inner) => {
                let selftype = self.find_self_type(*inner, selfty);
                self.hir.create_type(HirType::MutableRef(selftype))
            }
            Type::Nullable(inner) => {
                let selftype = self.find_self_type(*inner, selfty);
                self.hir.create_type(HirType::Nullable(selftype))
            }
            Type::Generic(_) => {
                panic!("Generics should not be handled. Cause i dont know how to handle them")
            }
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
            .find(|m| m.method_name == method_name);

        let Some(method) = method else {
            return Ok(None);
        };

        let self_sym = self.hir.intern_name("Self");
        let node = self.get_node(file_id);

        let mut args = Vec::with_capacity(method.arguments.len());
        let context = TypeContext::new(&method.type_params);
        for arg in &method.arguments {
            let ty = if let Some(name) = self.modules.referenced_name(arg.data.kind.data)
                && name == self_sym
            {
                self.find_self_type(arg.data.kind.data, struct_ty)
            } else {
                let (_, ty) = node.find_type(arg.data.kind, &context)?;
                ty
            };
            args.push(ty);
        }

        let return_type = if let Some(name) = self.modules.referenced_name(method.return_type.data)
            && name == self_sym
        {
            self.find_self_type(method.return_type.data, struct_ty)
        } else {
            let (_, ty) = node.find_type(method.return_type, &context)?;
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
                    generics: method.type_params.clone(),
                    args: Default::default(),
                    ty: func_ty,
                    statements: Vec::new(),
                    visibility: obj_decl.visibility,
                    external: obj_decl.external,
                    attributes: Vec::new(),
                    span: method.span,
                };
                let file = self.hir.get_or_create_file(obj_file_id);
                file.create_function(decl)
            },
        );

        self.hir
            .types_module
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
