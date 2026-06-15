use common::Span;
use slynx_lexer::tokens::TokenKind;

use crate::{
    ASTDeclaration, ASTDeclarationKind, ASTExpression, Parser, StyleBlock, StyleSheetStatement,
    StyleState, error::ParseError,
};

impl Parser {
    pub fn parse_style_state(&mut self) -> Result<StyleState, ParseError> {
        let mut states = Vec::new();
        loop {
            let TokenKind::Identifier(state_name) = self.expect_identifier()?.kind else {
                unreachable!()
            };
            states.push(state_name);
            if !matches!(self.peek()?.kind, TokenKind::Dot) {
                break;
            } else {
                self.expect(&TokenKind::Dot)?;
            }
        }
        let (duration, curve) = if let TokenKind::LParen = self.peek()?.kind
            && !states.is_empty()
        {
            self.eat()?;
            let duration = if self.peek()?.kind != TokenKind::RParen {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let curve = match self.peek()?.kind {
                TokenKind::Colon if self.peek_at(1)?.kind != TokenKind::RParen => {
                    let TokenKind::Identifier(ident) = self.expect_identifier()?.kind else {
                        unreachable!();
                    };
                    Some(ident)
                }
                _ => None,
            };
            self.expect(&TokenKind::RParen)?;
            (duration, curve)
        } else {
            (None, None)
        };
        Ok(StyleState {
            states,
            duration,
            transition_curve: curve,
        })
    }

    pub fn parse_style_block(&mut self, state: StyleState) -> Result<StyleBlock, ParseError> {
        let block_start = self.peek()?.span;
        let mut children_blocks = Vec::new();
        let mut properties = Vec::new();
        loop {
            if let TokenKind::RBrace = self.peek()?.kind {
                break;
            }

            match self.peek_at(1)?.kind {
                TokenKind::Dot | TokenKind::LParen | TokenKind::LBrace => {
                    let state = self.parse_style_state()?;
                    self.expect(&TokenKind::LBrace)?;
                    let block = self.parse_style_block(state)?;
                    children_blocks.push(block);
                }
                _ => {
                    let stmt = self.parse_named_expr()?;
                    properties.push(stmt);
                    if let TokenKind::Comma | TokenKind::SemiColon = self.peek()?.kind {
                        self.eat()?;
                    }
                }
            }
        }
        let block_end = self.expect(&TokenKind::RBrace)?.span;
        Ok(StyleBlock {
            state,
            properties,
            children: children_blocks,
            span: block_start.merge_with(block_end),
        })
    }

    pub fn parse_styles_statement(&mut self) -> Result<StyleSheetStatement, ParseError> {
        let styles_span = {
            let tk = self.expect_identifier()?;
            let TokenKind::Identifier(ref s) = tk.kind else {
                unreachable!();
            };
            if s != "styles" {
                return Err(ParseError::UnexpectedToken(
                    tk,
                    "Expected 'styles'".to_string(),
                ));
            }
            tk.span
        };
        self.expect(&TokenKind::LBrace)?;
        let mut styles = Vec::new();
        let style_block = self.parse_style_block(StyleState::new_base())?;
        let outspan = styles_span.merge_with(style_block.span);
        styles.push(style_block);
        Ok(StyleSheetStatement::Styles {
            styles,
            span: outspan,
        })
    }

    pub fn parse_stylesheet_statement(&mut self) -> Result<StyleSheetStatement, ParseError> {
        match self.peek()?.kind {
            TokenKind::Identifier(ref s) if s == "styles" => self.parse_styles_statement(),
            _ => {
                let out = self
                    .parse_statement()
                    .map(|arg| StyleSheetStatement::Statement(Box::new(arg)));
                self.expect(&TokenKind::SemiColon)?;
                out
            }
        }
    }

    pub fn parse_stylesheet_body(&mut self) -> Result<Vec<StyleSheetStatement>, ParseError> {
        let mut statements = Vec::new();
        loop {
            if let TokenKind::RBrace = self.peek()?.kind {
                break Ok(statements);
            }
            let stmt = self.parse_stylesheet_statement()?;
            statements.push(stmt);
            if let TokenKind::Comma = self.peek()?.kind {
                self.eat()?;
            }
        }
    }

    ///Parses the usages of the stylesheet. Thus, the `uses` until the `{` without consuming the `{`. The given `span` is the `uses` identifier span
    pub fn parse_usages(&mut self, _: Span) -> Result<Vec<ASTExpression>, ParseError> {
        let mut exprs = vec![];
        loop {
            let usage = self.parse_funcall()?;
            exprs.push(usage);
            match self.peek()?.kind {
                TokenKind::Comma => {
                    self.eat()?;
                }
                TokenKind::LBrace => {
                    break {
                        if exprs.is_empty() {
                            Err(ParseError::NoStyleUsagesProvided)
                        } else {
                            Ok(exprs)
                        }
                    };
                }
                _ => continue,
            }
        }
    }

    ///Parses a stylesheet. Thus the syntax `stylesheet Name(p1: T) uses Name2(f), F {...}`. The given `span` is the `stylesheet` keyword span
    pub fn parse_stylesheet(&mut self, span: Span) -> Result<ASTDeclaration, ParseError> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::LParen)?;
        let args = {
            let mut out = Vec::new();
            loop {
                if let TokenKind::RParen = self.peek()?.kind {
                    break out;
                }
                let arg = self.parse_typedname()?;
                out.push(arg);
                if let TokenKind::Comma = self.peek()?.kind {
                    self.eat()?;
                }
            }
        };

        self.expect(&TokenKind::RParen)?;
        let usages = if let TokenKind::Identifier(ref name) = self.peek()?.kind
            && name == "uses"
        {
            let span = self.eat()?.span;
            self.parse_usages(span)?
        } else {
            vec![]
        };
        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_stylesheet_body()?;
        let out = ASTDeclaration {
            attributes: Vec::new(),
            visibility: Default::default(),
            kind: ASTDeclarationKind::StyleSheet {
                name,
                args,
                usages,
                body,
            },
            external: false,
            span,
        };
        self.expect(&TokenKind::RBrace)?;
        Ok(out)
    }
}
