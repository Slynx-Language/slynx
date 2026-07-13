use common::{Span, Spanned, pool::DedupPoolId};
use module_loader::FileId;
use slynx_parser::{ASTStatement, FuncDeclaration};

use crate::{
    DeclarationId, HIRError, HirFunctionDeclaration, HirStatement, Result, SymbolPointer,
    VariableId,
    builders::{
        HirNode, HirQueueBuilder, PendantBody,
        expression::{ExpressionBuildResult, ExpressionBuilder},
    },
    context::HirSymbol,
    id::{AnyDeclarationId, AnyLocalDeclarationId, OwnerId},
};

pub struct HirFunctionBuilder {
    builder: ExpressionBuilder,
    target: DeclarationId<HirFunctionDeclaration>,
    args: Vec<VariableId>,
}

impl<'a> HirQueueBuilder<'a> {
    ///Hoists the given function, and then enqueues it so its body can be checked. On being processed, this function might generate more than simply the given `f` function since it will generate all the dependencies of `f` to work. Including impures
    pub(crate) fn enqueue_function(
        &self,
        f: &'a FuncDeclaration,
        node: HirNode<'_>,
    ) -> Result<DeclarationId<HirFunctionDeclaration>> {
        let name = self.modules.get_type(f.name.data).identifier;
        let signature = node.get_signature_of_function(f)?;
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
                    attributes: Vec::new(),
                };
                let file = self.hir.get_or_create_file(node.entry);
                file.create_function(decl)
            },
        );

        // Process attributes after the declaration is registered so we have the decl_id
        let decl_id =
            AnyDeclarationId::new(id.file_id, AnyLocalDeclarationId::Function(id.local_id));
        let attrs = super::attributes::process_attributes(self.hir, &f.attributes, decl_id);
        if !attrs.is_empty() {
            self.hir
                .get_file_mut(id.file_id)
                .declarations
                .functions
                .get_mut(id.local_id)
                .attributes = attrs;
        }

        self.bodies.send(PendantBody {
            func_id: id,
            body: &f.body,
            argument_names: names,
        });
        Ok(id)
    }

    ///Finds a function with the given `name` and returns it's id. If not found on the `requester` it tries to find on other files the requester imports. If not recognized by any, then hoists it properly
    #[allow(dead_code)]
    pub fn find_function_named(
        &'a self,
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
        } else if let Some((id, func)) = self.find_function_declaration(name, requester) {
            self.enqueue_function(func, self.get_node(id))
        } else {
            Err(HIRError::name_unrecognized(name, span))
        }
    }
}

impl HirFunctionBuilder {
    pub fn new(target: DeclarationId<HirFunctionDeclaration>) -> Self {
        Self {
            target,
            builder: ExpressionBuilder::new(OwnerId::Function(target)),
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
        self.args.push(id);
    }
    pub(crate) fn build_body(
        mut self,
        queue: &HirQueueBuilder<'_>,
        body: &[Spanned<DedupPoolId<ASTStatement>>],
    ) -> Result<ExpressionBuildResult> {
        let mut statements = Vec::new();
        let len = body.len();
        for (i, statement) in body.iter().enumerate() {
            if i + 1 == len {
                // Last statement: build raw data, apply implicit return, then insert
                let (data, span) = self.builder.build_statement_data(queue, statement)?;
                let data = match data {
                    HirStatement::Return { .. } => data,
                    HirStatement::Expression { expr } => HirStatement::Return { expr: Some(expr) },
                    _ => HirStatement::Return { expr: None },
                };
                let id = queue.hir.insert_statement(data);
                statements.push(span.make_spanned(id));
            } else {
                let stmt = self.builder.build_statement(queue, statement)?;
                statements.push(stmt);
            }
        }

        Ok(ExpressionBuildResult {
            args: self.args,
            statements,
        })
    }
}
