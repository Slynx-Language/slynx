use common::VisibilityModifier;
use slynx_parser::{GenericIdentifier, ObjectField, ObjectMethod};

use crate::{DeclarationId, HIRError, HirType, Result, SlynxHir, module_loader::FileId};

impl SlynxHir {
    pub(crate) fn create_empty_object(
        &self,
        file: FileId,
        name: &GenericIdentifier,
        fields: &[ObjectField],
        visibility: VisibilityModifier,
    ) -> DeclarationId {
        let name = self.intern_name(&name.identifier);
        let def_fields = fields
            .iter()
            .map(|f| self.intern_name(&f.name.name))
            .collect();
        let struct_ty = self
            .types_module
            .create_unnamed_type(HirType::new_struct(Vec::new()));
        let ty = self
            .types_module
            .create_type(name, HirType::new_ref(struct_ty));
        let local =
            self.get_file_mut(file)
                .declarations
                .register_object(name, ty, Vec::new(), visibility);
        self.types_module.objects.insert(ty, def_fields);
        DeclarationId::new(file, local)
    }
    /// Resolves an object declaration, filling in its field types and pushing the declaration.
    pub(crate) fn resolve_object(
        &self,
        name: &GenericIdentifier,
        fields: &[ObjectField],
    ) -> Result<()> {
        let mut fields = fields
            .iter()
            .map(|field| {
                let symbol_name = self.intern_name(&name.identifier);
                if self.intern_name(&field.name.name) == symbol_name {
                    Err(HIRError::recursive(symbol_name, field.name.span))
                } else {
                    let name = self.intern_name(&field.name.kind.identifier);
                    self.get_type_of_name(name, &field.name.span)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let identifier_symbol = self.intern_name(&name.identifier);
        let (_, declty) = self.find_declaration_by_name(&identifier_symbol, name.span)?;

        let rf = {
            let HirType::Reference { rf, .. } = *self.get_type(&declty) else {
                unreachable!("Type of custom object should be a reference to its real type");
            };
            rf
        };
        let HirType::Struct { fields: ty_field } = &mut *self.get_type_mut(rf) else {
            unreachable!("Type of object should be a Struct ty");
        };
        ty_field.append(&mut fields);

        Ok(())
    }

    pub(crate) fn lower_methods(
        &self,
        decl: DeclarationId,
        methods: &[ObjectMethod],
    ) -> Result<()> {
        let self_type = self.get_declaration_type(decl);
        for method in methods {
            let args = method
                .arguments
                .iter()
                .map(|arg| {
                    if arg.name == "Self" {
                        self_type
                    } else {
                        self.int32_type()
                    }
                })
                .collect();
            let return_type = if method.return_type.identifier == "Self" {
                self_type
            } else {
                self.int32_type()
            };
            let symbol = self.intern_name(&method.method_name.identifier);
            let ty = self
                .types_module
                .create_type(symbol, HirType::new_function(args, return_type));
            let local = self
                .get_file_mut(decl.file_id)
                .declarations
                .register_declaration_metadata(symbol, ty, VisibilityModifier::Private);
            self.get_file_mut(decl.file_id)
                .declarations
                .register_method(
                    decl.local_id,
                    symbol,
                    DeclarationId {
                        file_id: decl.file_id,
                        local_id: local,
                    },
                );
        }

        Ok(())
    }
}
