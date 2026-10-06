use module_loader::FileId;
use slynx_parser::{ComponentDeclaration, ComponentMemberKind, TypeContext};

use crate::{
    ComponentId, ComponentMemberDeclaration, DeclarationId, HIRError, HirComponentDeclaration,
    Result,
    builders::{
        HirQueueBuilder, PendantComponent,
        expression::{ExpressionBuilder, ExpressionDescriptor},
        lowering::lowerer::LowerTypeDeclarationDescriptor,
    },
    components::ComponentExpressionDescriptor,
    context::HirSymbol,
    id::{AnyLocalDeclarationId, OwnerId},
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

    pub(crate) fn enqueue_component(
        &self,
        component: &'a ComponentDeclaration,
        node: FileId,
    ) -> Result<DeclarationId<HirComponentDeclaration>> {
        let ast_type = self
            .lowerer
            .lookup
            .find_type(node, component.name)
            .ok_or_else(|| HIRError::type_unrecognized(component.name, component.span))?;
        let lowered = self.lowerer.materialize_type_declaration(
            self,
            LowerTypeDeclarationDescriptor {
                ast_type,
                context: &TypeContext::new(&component.generics.type_params),
                span: component.span,
            },
        )?;
        let (owner, ty) = (lowered.owner, lowered.term);

        let id = self.hir.symbols_registry.get_or_insert_component(
            HirSymbol::new(owner, component.name),
            || {
                let decl = HirComponentDeclaration {
                    name: component.name,
                    generics: self.lowerer.generic_parameters_of(
                        self,
                        &component.generics,
                        owner,
                        &TypeContext::new(&component.generics.type_params),
                    )?,
                    props: Vec::new(),
                    ty,
                    visibility: component.visibility,
                    attributes: Vec::new(),
                };
                let file = self.hir.store.get_or_create_file(node);
                Ok(file.create_component(decl))
            },
        )?;

        // Process attributes after the declaration is registered
        self.attach_attributes(
            id.owner,
            AnyLocalDeclarationId::Component(id.term),
            &component.attributes,
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
            builder: ExpressionBuilder::new(OwnerId::Component(target), None),
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
        let context = TypeContext::new(&component.generics.type_params);
        for member in &component.members {
            match &member.kind {
                ComponentMemberKind::Property {
                    name,
                    modifier,
                    rhs,
                    ..
                } => {
                    let rhs = if let Some(rhs) = rhs {
                        Some(
                            self.builder.build_expression(
                                queue,
                                ExpressionDescriptor {
                                    target: *rhs,
                                    expected: component_type
                                        .props()
                                        .get(prop_index)
                                        .map(|field| field.data.ty),
                                    context: &context,
                                },
                            )?,
                        )
                    } else {
                        None
                    };
                    decls.push(ComponentMemberDeclaration::Property {
                        name: *name,
                        modifier: *modifier,
                        index: prop_index,
                        value: rhs,
                        span: member.span,
                    });
                    prop_index += 1;
                }
                ComponentMemberKind::Child(c) => {
                    let expr = self.builder.build_component_expression(
                        queue,
                        ComponentExpressionDescriptor {
                            component: &c.data,
                            span: c.span,
                            context: &context,
                        },
                    )?;
                    decls.push(ComponentMemberDeclaration::Child(expr));
                }
            }
        }
        Ok(ComponentBuildResult { decls })
    }
}
