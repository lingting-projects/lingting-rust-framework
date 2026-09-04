use std::borrow::Borrow;

use specta::Types;
use specta_serde::PhasesFormat;
use specta_typescript::Typescript;

use crate::TypeMetadata;

use super::{TypescriptBuildResult, TypescriptBuilder, TypescriptResult};

/// 构建 Specta 类型声明。
pub struct TypescriptTypeBuilder {
    metadata: Vec<TypeMetadata>,
    _type_import_from: String,
}

impl TypescriptTypeBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            _type_import_from: ".".to_string(),
        }
    }

    pub fn types<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.push_types(metadata);
        self
    }

    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self._type_import_from = source.into();
        self
    }

    pub fn push_types<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }
}

impl TypescriptBuilder for TypescriptTypeBuilder {
    fn build(&self) -> TypescriptResult<TypescriptBuildResult> {
        let mut types = Types::default();
        for metadata in &self.metadata {
            types = metadata.register_type(types);
        }
        let dts = Typescript::default().export(&types, PhasesFormat)?;
        Ok(TypescriptBuildResult {
            js: String::new(),
            dts,
        })
    }
}

impl Default for TypescriptTypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
