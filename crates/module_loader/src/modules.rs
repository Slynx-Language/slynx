use std::{collections::HashMap, path::PathBuf};

use common::{
    FrontendSymbol, SymbolPointer, SymbolsModule,
    pool::{Pool, PoolId},
};
use slynx_parser::{
    ASTExpression, ASTPath, ASTStatement, AliasDeclaration, ComponentDeclaration,
    GenericIdentifier, ObjectDeclaration,
};

use crate::{FileId, SourceLoader, SourceNode};

pub struct Modules<'a> {
    pub(crate) loader: SourceLoader<'a>,
    pub(crate) modules: Vec<SourceNode>,
    pub(crate) paths: HashMap<PathBuf, FileId>,
}

pub enum ASTBuiltin {
    Boolean,
    Int(u8),
    Uint(u8),
    F16,
    F32,
    F64,
    Str,
}

pub enum ASTTypeKind<'a> {
    Struct(&'a ObjectDeclaration),
    Component(&'a ComponentDeclaration),
    Alias(&'a AliasDeclaration),
    Builtin(ASTBuiltin),
}

///Represents something that can be interpreted as a type on the AST
pub struct ASTType<'a> {
    pub owner: FileId,
    pub content: ASTTypeKind<'a>,
}

impl<'a> Modules<'a> {
    fn builtin_type(s: &str) -> Option<ASTBuiltin> {
        match s {
            "bool" => Some(ASTBuiltin::Boolean),
            "f16" => Some(ASTBuiltin::F16),
            "f32" => Some(ASTBuiltin::F32),
            "f64" => Some(ASTBuiltin::F64),
            "str" => Some(ASTBuiltin::Str),
            name if let ("uint", quantity) = name.split_at(4)
                && let Ok(value) = quantity.parse::<u8>() =>
            {
                Some(ASTBuiltin::Uint(value))
            }
            name if let ("int", quantity) = name.split_at(3)
                && let Ok(value) = quantity.parse::<u8>() =>
            {
                Some(ASTBuiltin::Int(value))
            }
            _ => None,
        }
    }

    pub fn entries(&self) -> &[SourceNode] {
        &self.modules
    }
    pub fn symbols(&self) -> &SymbolsModule<FrontendSymbol> {
        &self.loader.symbols
    }
    pub fn find_function_with_name(
        &self,
        module: &'a SourceNode,
        name: &str,
    ) -> Option<&'a slynx_parser::FuncDeclaration> {
        let target_symbol = self.loader.symbols.intern(name);
        module.func().iter().find(|name| {
            let t = self.loader.types.get(name.name.data);
            t.identifier == target_symbol
        })
    }
    pub fn get_expr(&self, expr: PoolId<ASTExpression>) -> &ASTExpression {
        self.loader.expressions.get(expr)
    }
    pub fn get_statement(&self, stmt: PoolId<ASTStatement>) -> &ASTStatement {
        self.loader.statements.get(stmt)
    }
    pub fn get_type(&self, ty: PoolId<GenericIdentifier>) -> &GenericIdentifier {
        self.loader.types.get(ty)
    }

    pub fn get_type_inside_module(
        &'a self,
        module: &'a SourceNode,
        name: SymbolPointer<FrontendSymbol>,
    ) -> Option<ASTType<'a>> {
        if let Some(kind) = Self::builtin_type(self.symbols().get_name(name)) {
            return Some(ASTType {
                owner: module.id,
                content: ASTTypeKind::Builtin(kind),
            });
        };
        for import in module.imports() {
            for usage in &import.usages {
                if let Some(_) = usage.alias {
                    let original = self.recreate_pathbuf(&import.path);
                    let file = self
                        .paths
                        .get(&original)
                        .expect("Expected original path to map properly to some file");
                    let t =
                        self.get_type_inside_module(&self.modules[file.as_raw() as usize], name);
                    if t.is_some() {
                        return t;
                    }
                }
            }
        }
        for strukt in module.object() {
            let strukt_name = self.get_type(strukt.name.data).identifier;
            if strukt_name == name {
                return Some(ASTType {
                    owner: module.id,
                    content: ASTTypeKind::Struct(strukt),
                });
            }
        }
        for component in module.component() {
            let component_name = self.get_type(component.name.data).identifier;
            if component_name == name {
                return Some(ASTType {
                    owner: module.id,
                    content: ASTTypeKind::Component(component),
                });
            }
        }
        for alias in module.alias() {
            let alias_name = self.get_type(alias.name.data).identifier;
            if alias_name == name {
                return Some(ASTType {
                    owner: module.id,
                    content: ASTTypeKind::Alias(alias),
                });
            }
        }
        None
    }

    fn recreate_pathbuf(&self, path: &ASTPath) -> PathBuf {
        let mut out = PathBuf::new();
        for module in &path.module_names {
            out.push(self.loader.symbols.get_name(*module));
        }
        out
    }
}
