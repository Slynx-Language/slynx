use std::{collections::HashMap, path::PathBuf};

use common::{FrontendSymbol, SymbolPointer, SymbolsModule, pool::DedupPoolId};
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
    Void,
    Boolean,
    Int(u8),
    Uint(u8),
    F16,
    F32,
    F64,
    Str,
    AnyComponent,
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
            "void" => Some(ASTBuiltin::Void),
            "int" => Some(ASTBuiltin::Int(32)),
            "uint" => Some(ASTBuiltin::Uint(32)),
            "Component" => Some(ASTBuiltin::AnyComponent),
            name if name.len() > 4
                && let ("uint", quantity) = name.split_at(4)
                && let Ok(value) = quantity.parse::<u8>() =>
            {
                Some(ASTBuiltin::Uint(value))
            }
            name if name.len() > 3
                && let ("int", quantity) = name.split_at(3)
                && let Ok(value) = quantity.parse::<u8>() =>
            {
                Some(ASTBuiltin::Int(value))
            }
            _ => None,
        }
    }

    pub fn get_entry(&self, id: FileId) -> &SourceNode {
        &self.modules[id.as_raw() as usize]
    }

    pub fn entries(&self) -> &[SourceNode] {
        &self.modules
    }
    pub fn symbols(&self) -> &SymbolsModule<FrontendSymbol> {
        self.loader.symbols
    }

    pub fn get_expr(&self, expr: DedupPoolId<ASTExpression>) -> &ASTExpression {
        self.loader.expressions.get(expr)
    }
    pub fn get_statement(&self, stmt: DedupPoolId<ASTStatement>) -> &ASTStatement {
        self.loader.statements.get(stmt)
    }
    pub fn get_type(&self, ty: DedupPoolId<GenericIdentifier>) -> &GenericIdentifier {
        self.loader.types.get(ty)
    }

    pub fn find_function_declaration(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
    ) -> Option<(FileId, &slynx_parser::FuncDeclaration)> {
        let module = &self.modules[module.as_raw() as usize];
        if let Some(v) = module.func().iter().find(|func| {
            let t = self.loader.types.get(func.name.data);
            t.identifier == name
        }) {
            return Some((module.id, v));
        }
        for import in module.imports() {
            for usage in &import.usages {
                let target = if let Some(name) = usage.alias {
                    name
                } else {
                    usage.content_name
                };
                let original = self.recreate_pathbuf(module.id, &import.path);
                let file = self
                    .paths
                    .get(&original)
                    .expect("Expected original path to properly map to some file");
                if let Some(func) = self.find_function_declaration(target, *file) {
                    return Some(func);
                };
            }
        }
        None
    }

    pub fn find_static_declaration(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
    ) -> Option<(FileId, &slynx_parser::StaticDeclaration)> {
        let module = &self.modules[module.as_raw() as usize];
        if let Some(v) = module.statics().iter().find(|statik| statik.name == name) {
            return Some((module.id, v));
        }
        for import in module.imports() {
            for usage in &import.usages {
                let target = if let Some(name) = usage.alias {
                    name
                } else {
                    usage.content_name
                };
                let original = self.recreate_pathbuf(module.id, &import.path);
                let file = self
                    .paths
                    .get(&original)
                    .expect("Expected original path to properly map to some file");
                if let Some(statik) = self.find_static_declaration(target, *file) {
                    return Some(statik);
                };
            }
        }
        None
    }

    pub fn find_type_inside_module(
        &'a self,
        module: FileId,
        name: SymbolPointer<FrontendSymbol>,
    ) -> Option<ASTType<'a>> {
        if let Some(kind) = Self::builtin_type(self.symbols().get_name(name)) {
            return Some(ASTType {
                owner: module,
                content: ASTTypeKind::Builtin(kind),
            });
        };

        let raw = module.as_raw() as usize;
        let module_ref = &self.modules[raw];

        if let Some(strukt) = module_ref.object().iter().find_map(|strukt| {
            let strukt_name = self.get_type(strukt.name.data).identifier;
            (strukt_name == name).then_some(ASTType {
                owner: module,
                content: ASTTypeKind::Struct(strukt),
            })
        }) {
            return Some(strukt);
        }
        if let Some(component) = module_ref.component().iter().find_map(|component| {
            let component_name = self.get_type(component.name.data).identifier;
            (component_name == name).then_some(ASTType {
                owner: module,
                content: ASTTypeKind::Component(component),
            })
        }) {
            return Some(component);
        }

        if let Some(alias) = module_ref.alias().iter().find_map(|alias| {
            let alias_name = self.get_type(alias.name.data).identifier;
            (alias_name == name).then_some(ASTType {
                owner: module,
                content: ASTTypeKind::Alias(alias),
            })
        }) {
            return Some(alias);
        }

        for import in module_ref.imports() {
            for usage in &import.usages {
                let target = match () {
                    _ if let Some(name) = usage.alias => name,
                    _ => usage.content_name,
                };

                let original = self.recreate_pathbuf(module, &import.path);

                let file = self
                    .paths
                    .get(&original)
                    .expect("Expected original path to map properly to some file");
                let t = self.find_type_inside_module(*file, target);
                if t.is_some() {
                    return t;
                }
            }
        }
        None
    }

    fn recreate_pathbuf(&self, entry: FileId, path: &ASTPath) -> PathBuf {
        let mut entry = self
            .paths
            .iter()
            .find_map(|v| (*v.1 == entry).then_some(v.0.clone()))
            .expect("File ID should map to some file");
        entry.pop(); //pops because it maps to some file, and we must get rid of the file
        for module in &path.module_names {
            entry.push(self.loader.symbols.get_name(*module));
        }
        entry.with_extension("slx")
    }
}
