use std::{collections::HashMap, path::PathBuf};

use common::{
    FrontendSymbol, SymbolPointer, SymbolsModule,
    pool::{DedupPoolId, PoolId},
};
use slynx_parser::{
    ASTExpression, ASTPath, ASTStatement, AliasDeclaration, ComponentDeclaration, EnumDeclaration,
    ObjectDeclaration, StaticDeclaration, Type, TypeContext,
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

pub enum ASTTypeKind {
    Struct(PoolId<ObjectDeclaration>),
    Component(PoolId<ComponentDeclaration>),
    Alias(PoolId<AliasDeclaration>),
    Enum(PoolId<EnumDeclaration>),
    Builtin(ASTBuiltin),
}

///Represents something that can be interpreted as a type on the AST
pub struct ASTType {
    pub owner: FileId,
    pub content: ASTTypeKind,
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
    pub fn get_type(&self, ty: DedupPoolId<Type>) -> &Type {
        self.loader.types.get(ty)
    }

    ///Retrieves the name of the inner type of the given `ty`. For example, if this is &Thing, this will return 'Thing' same as &mut Thing, and etc.
    pub fn referenced_name(&self, ty: DedupPoolId<Type>) -> Option<SymbolPointer<FrontendSymbol>> {
        match self.get_type(ty) {
            Type::Plain(generic) => Some(generic.identifier),
            Type::Reference(ty) => self.referenced_name(*ty),
            Type::MutableReference(ty) => self.referenced_name(*ty),
            _ => None,
        }
    }

    pub fn type_name(
        &self,
        ty: DedupPoolId<Type>,
        context: &TypeContext<'_>,
    ) -> SymbolPointer<FrontendSymbol> {
        slynx_parser::type_name(
            self.loader.types,
            self.loader.symbols,
            self.loader.expressions,
            ty,
            context.generic_names,
        )
    }

    pub fn find_in_modules<T>(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
        finder: &dyn Fn(&SourceNode, SymbolPointer<FrontendSymbol>) -> Option<T>,
    ) -> Option<(FileId, T)> {
        let module = &self.modules[module.as_raw() as usize];
        if let Some(v) = finder(module, name) {
            return Some((module.id, v));
        }
        for import in module.imports().iter() {
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
                if let Some(func) = self.find_in_modules(target, *file, finder) {
                    return Some(func);
                };
            }
        }
        None
    }

    ///Finds a function with the given name available in the given module. Returns the file that owns the function and the index of the function in the module.
    pub fn find_function_declaration(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
    ) -> Option<(FileId, usize)> {
        self.find_in_modules(name, module, &|module, name| {
            module.func().iter().position(|func| func.name == name)
        })
    }

    ///Finds a static variable with the given name available in the given module. Returns the file that owns the static variable and a reference to it.
    pub fn find_static_declaration(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
    ) -> Option<(FileId, &StaticDeclaration)> {
        let module = &self.modules[module.as_raw() as usize];
        if let Some(statik) = module.statics().iter().find(|statik| statik.name == name) {
            return Some((module.id, statik));
        }
        for import in module.imports().iter() {
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
                }
            }
        }
        None
    }

    ///Finds a type (struct, component, alias, enum or builtin) with the given name available in the given module or in the modules it imports.
    pub fn find_type(
        &self,
        module: FileId,
        name: SymbolPointer<FrontendSymbol>,
    ) -> Option<ASTType> {
        if let Some(kind) = Self::builtin_type(self.symbols().get_name(name)) {
            return Some(ASTType {
                owner: module,
                content: ASTTypeKind::Builtin(kind),
            });
        };
        self.find_in_modules(name, module, &|module, name| {
            if let Some((id, _)) = module
                .object()
                .iter()
                .with_ids()
                .find(|(_, strukt)| strukt.name == name)
            {
                return Some(ASTTypeKind::Struct(id));
            }
            if let Some((id, _)) = module
                .component()
                .iter()
                .with_ids()
                .find(|(_, component)| component.name == name)
            {
                return Some(ASTTypeKind::Component(id));
            }
            if let Some((id, _)) = module
                .alias()
                .iter()
                .with_ids()
                .find(|(_, alias)| alias.name == name)
            {
                return Some(ASTTypeKind::Alias(id));
            }
            if let Some((id, _)) = module
                .enums()
                .iter()
                .with_ids()
                .find(|(_, enumer)| enumer.name == name)
            {
                return Some(ASTTypeKind::Enum(id));
            }
            None
        })
        .map(|(owner, content)| ASTType { owner, content })
    }

    ///Finds an enum that declares a variant with the given `name`, available in
    ///the given module or in the modules it imports. Returns the file that owns
    ///the enum, the id of the enum within that file, and the index of the
    ///variant within the enum.
    pub fn find_enum_variant(
        &self,
        name: SymbolPointer<FrontendSymbol>,
        module: FileId,
    ) -> Option<(FileId, PoolId<EnumDeclaration>, usize)> {
        self.find_in_modules(name, module, &|module, name| {
            module.enums().iter().with_ids().find_map(|(id, enumer)| {
                enumer
                    .variants
                    .iter()
                    .position(|variant| variant.name.data == name)
                    .map(|index| (id, index))
            })
        })
        .map(|(owner, (id, index))| (owner, id, index))
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
