use common::Span;
use module_loader::{ASTTypeKind, FileId};
use slynx_parser::{ComponentDeclaration, ComponentMemberKind};

use crate::{
    ComponentId, ComponentMemberDeclaration, DeclarationId, HIRError, HirComponentDeclaration,
    Result, SymbolPointer,
    builders::{HirNode, HirQueueBuilder, PendantComponent, expression::ExpressionBuilder},
    context::HirSymbol,
    id::OwnerId,
};

pub struct ComponentBuildResult {
    pub(crate) decls: Vec<ComponentMemberDeclaration>,
}

pub struct ComponentBuilder {
    target: DeclarationId<HirComponentDeclaration>,
    builder: ExpressionBuilder,
}

impl<'a> HirQueueBuilder<'a> {
    /// Resolve (or return memoized) the body of a component.
    ///
    /// The body includes property default expressions and the child component tree.
    /// This NEVER resolves the body of child components — only the signature of
    /// components referenced by children may be resolved as a side effect.
    pub fn component_body(
        &self,
        id: ComponentId,
        queue: &crate::builders::HirQueueBuilder,
        component_decl: &slynx_parser::ComponentDeclaration,
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        // Already resolved? (scoped to drop dashmap Ref before body resolution)
        {
            let comp_ref = self.hir.get_component(id);
            let comp = comp_ref.value();
            if !comp.props.is_empty() {
                return Ok(comp.props.clone());
            }
        }

        // Mark body as in-progress (simple cycle guard — body never demands body of another)
        if !queue.bodies_in_progress.insert(id) {
            return Err(HIRError::cyclic_component_body(id, component_decl.span));
        }

        let result = ComponentBuilder::new(id).resolve_body(queue, component_decl);

        queue.bodies_in_progress.remove(&id);
        result.map(|r| r.decls)
    }
    ///Finds a component with the given `name` on-demand, hoisting it if needed.
    ///Mirrors the pattern of `find_function_named`.
    pub fn find_component_named(
        &'a self,
        name: SymbolPointer,
        requester: FileId,
        span: Span,
    ) -> Result<ComponentId> {
        // 1. Already hoisted in symbol registry?
        if let Some(comp) = self
            .hir
            .find_component_by_symbol(HirSymbol::new(requester, name))
        {
            return Ok(comp);
        }

        // 2. Already exists in the requester's file pool?
        if let Some(id) = self.hir.get_file(requester).find_component_with_name(name) {
            return Ok(id);
        }

        // 3. Search AST through imports and hoist on-demand
        if let Some(ast_type) = self.modules.find_type_inside_module(requester, name) {
            match ast_type.content {
                ASTTypeKind::Component(component) => {
                    let out = self.enqueue_component(component, ast_type.owner)?;
                    return Ok(out);
                }
                _ => {
                    return Err(HIRError::not_a_component(name, span));
                }
            }
        }

        // 4. Not found anywhere
        Err(HIRError::name_unrecognized(name, span))
    }
    pub(crate) fn enqueue_component(
        &self,
        component: &'a ComponentDeclaration,
        node: FileId,
    ) -> Result<DeclarationId<HirComponentDeclaration>> {
        let node = self.get_node(node);
        let (owner, ty) = node.find_type(component.name)?;
        let name = self.modules.get_type(component.name.data).identifier;
        let id = self.hir.symbols_registry.get_or_insert_component(
            HirSymbol::new(owner, name),
            || {
                let decl = HirComponentDeclaration {
                    name,
                    props: Vec::new(),
                    ty,
                    visibility: component.visibility,
                };
                let file = self.hir.get_or_create_file(node.entry);
                Ok(file.create_component(decl))
            },
        )?;
        self.components.send(PendantComponent {
            owner: id,
            component,
        });
        Ok(id)
    }
}

impl ComponentBuilder {
    pub fn new(target: DeclarationId<HirComponentDeclaration>) -> Self {
        Self {
            target,
            builder: ExpressionBuilder::new(OwnerId::Component(target)),
        }
    }

    pub fn resolve_body(
        mut self,
        queue: &HirQueueBuilder,
        component: &ComponentDeclaration,
    ) -> Result<ComponentBuildResult> {
        let mut decls = Vec::new();
        let raw_type = queue
            .hir
            .view(queue.hir.get_component(self.target).value().ty);
        let Some(component_type) = raw_type.is_component() else {
            unreachable!("Wtf. Component declaration should contain a component type");
        };

        let mut prop_index = 0;
        for member in &component.members {
            match &member.kind {
                ComponentMemberKind::Property { rhs, .. } => {
                    let rhs = if let Some(rhs) = rhs {
                        Some(self.builder.build_expression(
                            queue,
                            *rhs,
                            component_type.props().get(prop_index).cloned(),
                        )?)
                    } else {
                        None
                    };
                    decls.push(ComponentMemberDeclaration::Property {
                        index: prop_index,
                        value: rhs,
                        span: member.span,
                    });
                    prop_index += 1;
                }
                ComponentMemberKind::Child(c) => {
                    let expr = self
                        .builder
                        .build_component_expression(queue, &c.data, c.span)?;
                    decls.push(ComponentMemberDeclaration::Child(expr));
                }
            }
        }
        Ok(ComponentBuildResult { decls })
    }
}
