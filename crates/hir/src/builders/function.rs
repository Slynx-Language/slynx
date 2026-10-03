use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use module_loader::FileId;
use slynx_parser::{ASTFunction, ASTStatement, FuncDeclaration, Type, TypeContext};

use crate::{
    DeclarationId, HIRError, HirFunctionDeclaration, HirStatement, Result, SymbolPointer,
    VariableId,
    builders::{
        HirQueueBuilder, PendantFunction,
        expression::{ExpressionBuildResult, ExpressionBuilder},
    },
    context::HirSymbol,
    id::{AnyLocalDeclarationId, OwnerId},
    term::{Term, TermId},
};

pub struct HirFunctionBuilder {
    builder: ExpressionBuilder,
    target: DeclarationId<HirFunctionDeclaration>,
    args: Vec<VariableId>,
}

impl<'a> HirQueueBuilder<'a> {
    pub(crate) fn insert_method_declaration<T: ASTFunction>(
        &self,
        entry: FileId,
        method: &'a T,
        self_type: TermId,
        visibility: VisibilityModifier,
        external: bool,
        declaration_name: SymbolPointer,
        register_as_inherent: bool,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let context = TypeContext::new(method.type_params());
        let lower_type = |ty: Spanned<DedupPoolId<Type>>| {
            self.lowerer
                .lower_type_with_self(self, entry, ty, &context, self_type)
                .map(|owned| owned.term)
        };
        let args = method
            .arguments()
            .iter()
            .map(|arg| lower_type(arg.data.kind))
            .collect::<Result<Vec<_>>>()?;
        let return_type = lower_type(method.return_type())?;
        let function_type = self.hir.types.create_function_type(args, return_type);
        let declaration = HirFunctionDeclaration {
            name: declaration_name,
            generics: method.type_params().to_vec(),
            args: Default::default(),
            ty: function_type,
            statements: Vec::new(),
            visibility,
            external,
            attributes: Vec::new(),
            span: method.span(),
        };
        let make_declaration = || {
            self.hir
                .store
                .get_or_create_file(entry)
                .create_function(declaration)
        };
        let declaration_id = if register_as_inherent {
            self.hir
                .symbols_registry
                .get_or_insert_function(HirSymbol::new(entry, declaration_name), make_declaration)
        } else {
            make_declaration()
        };

        if register_as_inherent {
            self.hir
                .types
                .create_method(self_type, method.method_name(), declaration_id);
        }

        if !external {
            let argument_names = method
                .arguments()
                .iter()
                .map(|arg| arg.data.name.data)
                .collect();
            self.bodies.send(PendantFunction {
                context,
                func_id: declaration_id,
                body: method.body(),
                argument_names,
                self_type: Some(self_type),
            });
        }

        Ok(declaration_id)
    }

    ///Hoists the given function, and then enqueues it so its body can be checked. On being processed, this function might generate more than simply the given `f` function since it will generate all the dependencies of `f` to work. Including impures
    pub(crate) fn enqueue_function(
        &self,
        f: &'a FuncDeclaration,
        owner: FileId,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let signature = self.lowerer.resolve_signature_of_function(self, owner, f)?;
        let names = f.args.iter().map(|arg| arg.data.name.data).collect();
        let id =
            self.hir
                .symbols_registry
                .get_or_insert_function(HirSymbol::new(owner, f.name), || {
                    let decl = HirFunctionDeclaration {
                        name: f.name,
                        generics: f.type_params.clone(),
                        args: Default::default(),
                        ty: signature,
                        statements: Vec::new(),
                        visibility: f.visibility,
                        external: f.external,
                        attributes: Vec::new(),
                        span: f.span,
                    };
                    let file = self.hir.store.get_or_create_file(owner);
                    file.create_function(decl)
                });

        // Process attributes after the declaration is registered so we have the decl_id
        self.attach_attributes(
            id.file_id,
            AnyLocalDeclarationId::Function(id.local_id),
            &f.attributes,
        )?;

        self.bodies.send(PendantFunction {
            context: TypeContext::new(&f.type_params),
            func_id: id,
            body: &f.body,
            argument_names: names,
            self_type: None,
        });
        Ok(id)
    }

    ///Finds a function with the given `name` and returns it's id. If not found on the `requester` it tries to find on other files the requester imports. If not recognized by any, then hoists it properly
    pub fn find_function_named(
        &self,
        name: SymbolPointer,
        requester: FileId,
        span: Span,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        if let Some(func) = self
            .hir
            .find_function_by_symbol(HirSymbol::new(requester, name))
        {
            Ok(func)
        } else if let Some(func) = self.hir.get_file(requester).find_function_with_name(name) {
            Ok(func)
        } else if let Some(id) = self.lowerer.lookup.find_function(name, requester) {
            let func = self.modules.get_entry(id.owner).func().get(id.term);
            self.enqueue_function(func, id.owner)
        } else {
            Err(HIRError::name_unrecognized(name, span))
        }
    }
}

impl HirFunctionBuilder {
    pub fn new(target: DeclarationId<HirFunctionDeclaration>, self_type: Option<TermId>) -> Self {
        Self {
            target,
            builder: ExpressionBuilder::new(OwnerId::Function(target), self_type),
            args: Vec::new(),
        }
    }
    pub(crate) fn create_argument(
        &mut self,
        queue: &HirQueueBuilder,
        name: SymbolPointer,
        arg_index: u8,
    ) {
        let (id, ty) = queue
            .hir
            .view(self.target)
            .get_argument(arg_index)
            .expect("Argument index should be < function argument count");
        self.builder.create_mapped_variable(name, id, false, ty);
        queue.hir.store.variable_names.insert(id, name);
        self.args.push(id);
    }
    pub(crate) fn build_body(
        mut self,
        queue: &HirQueueBuilder<'_>,
        body: &[Spanned<DedupPoolId<ASTStatement>>],
        context: &TypeContext,
    ) -> Result<ExpressionBuildResult> {
        let mut contains_return = false;
        let statements = {
            let mut statements = Vec::new();
            let len = body.len();

            for (i, statment) in body.iter().enumerate() {
                if contains_return {
                    break;
                }
                let (statment, span) = self
                    .builder
                    .build_statement_data(queue, statment, context)?;
                let statment = if i + 1 == len
                    && let HirStatement::Expression { expr } = statment
                {
                    HirStatement::Return { expr: Some(expr) }
                } else {
                    statment
                };
                contains_return = matches!(statment, HirStatement::Return { .. });
                let stmt = queue.hir.store.insert_statement(statment);
                statements.push(span.make_spanned(stmt));
            }
            statements
        };
        let func_view = queue.hir.view(self.target);
        if !func_view.raw_declaration().external
            && !contains_return
            && func_view.return_type() != queue.hir.types.create_type(Term::void_type())
        {
            Err(HIRError::missing_return(func_view.raw_declaration().span))
        } else {
            Ok(ExpressionBuildResult {
                args: self.args,
                statements,
            })
        }
    }
}
