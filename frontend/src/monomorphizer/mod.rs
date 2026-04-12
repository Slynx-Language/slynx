use std::collections::{HashMap, HashSet};

use color_eyre::eyre::Result;

use crate::hir::{
    SlynxHir, TypeId,
    error::{HIRError, HIRErrorKind},
    types::{HirType, TypesModule},
};

///A struct that handles all the monomorphization on the code
pub struct Monomorphizer {
    reference_cache: HashMap<TypeId, TypeId>,
}

impl Monomorphizer {
    pub fn resolve(hir: &SlynxHir, types_module: &mut TypesModule) -> Result<()> {
        let mut this = Self {
            reference_cache: HashMap::new(),
        };
        for decl in hir.declarations.iter() {
            this.resolve_reference(hir, decl.ty, decl.span.clone(), types_module)?;
        }
        for (key, value) in this.reference_cache {
            let HirType::Reference { rf, .. } = types_module.get_type_mut(&key) else {
                continue;
            };
            *rf = value;
        }
        Ok(())
    }
    /// Resolves a reference. If the provided `id` is a reference to a concrete type, doesnt do anything, otherwise(thus, a reference)
    /// to another reference) it resolves it to make the reference point to the concrete type. This only caches it for later mutability
    pub fn resolve_reference(
        &mut self,
        hir: &SlynxHir,
        id: TypeId,
        span: common::ast::Span,
        types_module: &TypesModule,
    ) -> Result<()> {
        let mut current = id;
        let mut visited = HashSet::from([id]);
        while let HirType::Reference { rf, .. } = types_module.get_type(&current)
            && let HirType::Reference { .. } = types_module.get_type(rf)
        {
            if !visited.insert(*rf) {
                let ty = types_module
                    .get_type_name(&id)
                    .map(|symbol| hir.symbols_module.get_name(*symbol).to_string())
                    .unwrap_or_else(|| format!("type id {}", id.as_raw()));

                return Err(HIRError {
                    kind: HIRErrorKind::RecursiveType { ty },
                    span,
                }
                .into());
            }
            current = *rf;
        }
        if current != id {
            self.reference_cache.insert(id, current);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Monomorphizer;
    use crate::{
        checker::TypeChecker,
        hir::{
            SlynxHir,
            error::{HIRError, HIRErrorKind},
        },
        lexer::Lexer,
        parser::Parser,
    };

    fn load_hir_from_source(source: &str) -> SlynxHir {
        let tokens = Lexer::tokenize(source).expect("source should tokenize");
        let declarations = Parser::new(tokens)
            .parse_declarations()
            .expect("source should parse");
        let mut hir = SlynxHir::new();
        hir.generate(declarations).expect("HIR should generate");
        hir
    }

    #[test]
    fn rejects_cyclic_aliases_in_monomorphization() {
        let mut hir = load_hir_from_source(
            r#"
            alias A = B;
            alias B = A;

            func main(): void {}
            "#,
        );
        let mut types_module = TypeChecker::check(&mut hir).expect("type checking should pass");

        let err = Monomorphizer::resolve(&hir, &mut types_module)
            .expect_err("cyclic aliases should fail during monomorphization");
        let hir_error = err
            .downcast_ref::<HIRError>()
            .expect("error should come from the HIR layer");

        match &hir_error.kind {
            HIRErrorKind::RecursiveType { ty } => assert_eq!(ty, "A"),
            other => panic!("expected RecursiveType, got {other:?}"),
        }
    }
}
