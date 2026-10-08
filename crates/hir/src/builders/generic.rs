use common::{Spanned, pool::DedupPoolId};
use module_loader::{ASTType, FileId};
use slynx_parser::Type;

use crate::{HIRError, HirQueueBuilder, Result, error::InvalidTypeReason};

impl<'a> HirQueueBuilder<'a> {
    ///Asserts the given `ty` has the correct number of generic parameters of its type definition. A struct defined with 3 generic parameters will only pass if the type referecing it also contains 3 generic parameters
    pub(crate) fn assert_concrete_type_generic_count(
        &self,
        owner: FileId,
        ty: Spanned<DedupPoolId<Type>>,
    ) -> Result<()> {
        match self.modules.get_type(ty.data) {
            Type::Plain(identifier)
                if let Some(ASTType { owner, content }) =
                    self.lowerer.lookup.find_type(owner, identifier.identifier) =>
            {
                let generic_count = self.modules.generic_count(ASTType { owner, content });
                if generic_count != identifier.generic.len() {
                    let reason = if identifier.generic.len() < generic_count {
                        InvalidTypeReason::MissingGeneric
                    } else {
                        InvalidTypeReason::IncorrectUsage
                    };
                    return Err(HIRError::invalid_type(
                        identifier.identifier,
                        reason,
                        ty.span,
                    ));
                }
                for generic in &identifier.generic {
                    self.assert_concrete_type_generic_count(owner, *generic)?;
                }
            }
            Type::Plain(identifier) => {
                //couldnt find any type
                return Err(HIRError::type_unrecognized(identifier.identifier, ty.span));
            }
            Type::Array(element, _) | Type::Vector(element) => {
                self.assert_concrete_type_generic_count(owner, ty.span.make_spanned(*element))?;
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.assert_concrete_type_generic_count(owner, ty.span.make_spanned(*inner))?;
            }
        }
        Ok(())
    }
}
