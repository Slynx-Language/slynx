use crate::{
    ComponentMemberDeclaration, ComponentProperty, DeclarationId, HirDeclaration,
    HirDeclarationKind, Result, SlynxHir, TypeId,
    error::{HIRError, HIRErrorKind},
    model::{
        HirComponentExpression, HirSpecializedComponentExpression, HirType, PropertyExpression,
    },
    module_loader::FileId,
};
///Module that implements anything related Specialized Component on the HIR
use common::Span;
use common::VisibilityModifier;
use slynx_parser::{
    ComponentExpression, ComponentMember, ComponentMemberKind, ComponentMemberValue,
    GenericIdentifier,
};

impl SlynxHir {
    /// Hoists a component declaration by registering its property layout without resolving children.
    pub(crate) fn hoist_component(
        &self,
        file: FileId,
        name: &GenericIdentifier,
        members: &[ComponentMember],
        visibility: VisibilityModifier,
    ) -> Result<DeclarationId> {
        let props = members
            .iter()
            .filter_map(|member| match &member.kind {
                ComponentMemberKind::Property {
                    name,
                    modifier,
                    ty: Some(generic),
                    ..
                } => {
                    let name = self.intern_name(name);
                    let ty_name = self.intern_name(&generic.identifier);
                    let ty = match self.get_type_of_name(ty_name, &member.span) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
                    Some(Ok(ComponentProperty::new(*modifier, name, ty)))
                }
                ComponentMemberKind::Property {
                    name,
                    modifier,
                    ty: None,
                    ..
                } => {
                    let name = self.intern_name(name);
                    Some(Ok(ComponentProperty::new(
                        *modifier,
                        name,
                        self.infer_type(),
                    )))
                }
                ComponentMemberKind::Child(_) => None,
            })
            .collect::<Result<Vec<_>>>()?;
        let symbol = self.intern_name(&name.identifier);
        let ty = self
            .types_module
            .create_type(symbol, HirType::new_component(props));
        let local = self
            .get_file_mut(file)
            .declarations
            .register_declaration_metadata(symbol, ty, visibility);
        Ok(DeclarationId::new(file, local))
    }

    /// Resolves the member definitions of a component body into [`ComponentMemberDeclaration`]s.
    pub(crate) fn resolve_component_defs(
        &self,
        fileid: FileId,
        def: &[ComponentMember],
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        let mut out = Vec::with_capacity(def.len());
        let mut prop_idx = 0;
        for def in def {
            match &def.kind {
                ComponentMemberKind::Property { ty, rhs, name, .. } => {
                    let ty = if let Some(ty) = ty {
                        let symbol = self.intern_name(&ty.identifier);
                        self.get_type_of_name(symbol, &ty.span)?
                    } else {
                        self.infer_type()
                    };
                    let rhs = if let Some(rhs) = rhs {
                        Some(self.generate_expression(fileid, rhs, Some(ty))?)
                    } else {
                        None
                    };
                    out.push(ComponentMemberDeclaration::new_property(
                        prop_idx, rhs, def.span,
                    ));
                    let name = self.intern_name(name);
                    self.create_variable(fileid, name, ty, &def.span)?;
                    prop_idx += 1;
                }
                ComponentMemberKind::Child(child) => {
                    let component = self.resolve_component_expression(fileid, child)?;
                    out.push(ComponentMemberDeclaration::Child(component));
                }
            }
        }
        Ok(out)
    }

    ///Resolves a component declaration that contains the given `members` and the given `name`
    pub(crate) fn resolve_component_declaration(
        &self,
        fileid: FileId,
        members: &[ComponentMember],
        name: &GenericIdentifier,
        span: Span,
    ) -> Result<()> {
        let symbol = self.intern_name(&name.identifier);
        let defs = self.resolve_component_defs(fileid, members)?;

        let (decl, ty) = self.find_declaration_by_name(&symbol, span)?;

        let mut file = self.get_file_mut(fileid);
        file.scopes.enter_scope();
        file.create_declaration(HirDeclaration {
            id: decl,
            kind: HirDeclarationKind::ComponentDeclaration {
                props: defs,
                name: symbol,
            },
            ty,
            span,
            visibility: Default::default(),
        });
        file.scopes.exit_scope();
        Ok(())
    }

    fn resolve_component_member(
        &self,
        file: FileId,
        member: &ComponentMemberValue,
        ty: TypeId,
        properties: &mut Vec<PropertyExpression>,
        children: &mut Vec<HirComponentExpression>,
    ) -> Result<()> {
        match member {
            ComponentMemberValue::Assign {
                prop_name,
                rhs,
                span,
            } => {
                let interned_name = self.intern_name(prop_name);
                let props = {
                    let reader = self.get_type(&ty);
                    let HirType::Component { props } = &*reader else {
                        unreachable!("The type should be a component instead");
                    };
                    props.clone()
                };
                match props.iter().position(|prop| prop.name() == interned_name) {
                    None => Err(HIRError::name_unrecognized(interned_name, *span)),
                    Some(index)
                        if matches!(
                            props[index].visibility(),
                            VisibilityModifier::Private | VisibilityModifier::ChildrenPublic
                        ) =>
                    {
                        Err(HIRError::not_visible_property(interned_name, *span))
                    }

                    Some(index) => {
                        let expr =
                            self.generate_expression(file, rhs, Some(*props[index].prop_type()))?;
                        properties.push(PropertyExpression::new(index, expr));
                        Ok(())
                    }
                }
            }
            ComponentMemberValue::Child(child) => {
                //By now this won't track whether it can or cannot have children, since a method better than 'children' might be implemented in the future.
                let child = self.resolve_component_expression(file, child)?;
                children.push(child);
                Ok(())
            }
        }
    }

    /// Resolves the provided values on a component. The `ty` is the type of the component we are resolving it
    pub(crate) fn resolve_component_members(
        &self,
        file: FileId,
        members: &[ComponentMemberValue],
        ty: TypeId,
    ) -> Result<(Vec<PropertyExpression>, Vec<HirComponentExpression>)> {
        let mut properties = Vec::with_capacity(members.len());
        let mut children = Vec::with_capacity(members.len());
        for member in members {
            self.resolve_component_member(file, member, ty, &mut properties, &mut children)?;
        }

        Ok((properties, children))
    }

    /// Resolves the provided `values` as members of the `Text` specialized component.
    ///
    /// Expects exactly one `text` property assignment and no children.
    pub(crate) fn resolve_specialize_text(
        &self,
        file: FileId,
        values: &[ComponentMemberValue],
        span: &Span,
    ) -> Result<HirSpecializedComponentExpression> {
        let mut text = None;
        let mut style = None;
        for value in values {
            match value {
                ComponentMemberValue::Assign { prop_name, rhs, .. }
                    if prop_name == HirSpecializedComponentExpression::RESERVED_TEXT =>
                {
                    text = Some(self.generate_expression(file, rhs, None)?)
                }
                ComponentMemberValue::Assign { prop_name, rhs, .. }
                    if prop_name == HirSpecializedComponentExpression::RESERVED_STYLE =>
                {
                    style = Some(self.resolve_style_usage(file, rhs)?)
                }
                ComponentMemberValue::Assign {
                    prop_name, span, ..
                } => {
                    let intern = self.intern_name(prop_name);
                    return Err(HIRError::type_unrecognized(intern, *span));
                }
                ComponentMemberValue::Child(_) => {
                    return Err(HIRError {
                        kind: HIRErrorKind::InvalidChild,
                        span: *span,
                    });
                }
            }
        }
        match text {
            Some(text) => Ok(HirSpecializedComponentExpression::new_text(text, style)),
            None => {
                let properties = vec![self.intern_name("text")];
                Err(HIRError::missing_properties(properties, *span))
            }
        }
    }

    ///Resolves the provided `children` knowning it is a specialized div component
    pub(crate) fn resolve_specialized_div(
        &self,
        file: FileId,
        children: &[ComponentMemberValue],
        _: &Span,
    ) -> Result<HirSpecializedComponentExpression> {
        let style = {
            let mut c = None;
            for child in children {
                if let ComponentMemberValue::Assign {
                    prop_name,
                    rhs,
                    span,
                } = child
                {
                    if prop_name == HirSpecializedComponentExpression::RESERVED_STYLE {
                        c = Some(self.resolve_style_usage(file, rhs)?);
                    } else {
                        let prop = self.intern_name(prop_name);
                        return Err(HIRError::property_unrecognized(vec![prop], *span));
                    }
                }
            }
            c
        };

        let children = children
            .iter()
            .filter_map(|c| match c {
                ComponentMemberValue::Assign { .. } => None,
                ComponentMemberValue::Child(c) => Some(self.resolve_component_expression(file, c)),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(HirSpecializedComponentExpression::new_div(children, style))
    }
    ///Tries to resolve the given `child` as, either a specialized component, or a normal user defined component
    pub(crate) fn try_resolve_specialized<'a>(
        &self,
        file: FileId,
        child: &'a ComponentExpression,
    ) -> (
        Option<Result<HirSpecializedComponentExpression>>,
        Option<&'a ComponentExpression>,
    ) {
        match (child.name.identifier.as_str(), &child.name.generic) {
            ("Text", None) => (
                Some(self.resolve_specialize_text(file, &child.values, &child.span)),
                None,
            ),
            ("Div", None) => (
                Some(self.resolve_specialized_div(file, &child.values, &child.span)),
                None,
            ),
            _ => (None, Some(child)),
        }
    }

    ///Resolves the provided `component` expression. If it's a specialized one, resolves as a `SpecializedComponent`, otherwise as a normal 'Component'
    pub(crate) fn resolve_component_expression(
        &self,
        file: FileId,
        component: &ComponentExpression,
    ) -> Result<HirComponentExpression> {
        match self.try_resolve_specialized(file, component) {
            (Some(spec), None) => spec.map(HirComponentExpression::Specialized),
            (None, Some(component)) => {
                let name = self.intern_name(&component.name.identifier);
                let id = self.get_type_of_name(name, &component.span)?;
                let (properties, children) =
                    self.resolve_component_members(file, &component.values, id)?;
                Ok(HirComponentExpression::new_normal(
                    id,
                    properties,
                    children,
                    component.span,
                ))
            }
            (_, _) => unreachable!(
                "Try resolve specialized is bugged. This should literally never happen"
            ),
        }
    }
}
