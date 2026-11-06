use std::borrow::Borrow;

use crate::TypeMetadata;

use super::common::{
    RustBuildResult, RustBuilder, RustDependency, RustResult, build_error, join_sections,
};
use super::utils::type_short_name;

/// 构建 Rust 类型定义模块。
pub struct RustTypeBuilder {
    metadata: Vec<TypeMetadata>,
    type_import_from: Vec<String>,
}

impl RustTypeBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            type_import_from: Vec::new(),
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

    pub fn push_types<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    /// 追加一条 `use <source>;` 语句，可多次调用。
    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from.push(source.into());
        self
    }
}

impl RustBuilder for RustTypeBuilder {
    fn build(&self) -> RustResult<RustBuildResult> {
        let mut metadata = self.metadata.clone();
        metadata.sort_by_key(|metadata| metadata.full_name());

        let mut names = std::collections::BTreeSet::new();
        let mut sections = self
            .type_import_from
            .iter()
            .map(|source| format!("use {source};"))
            .collect::<Vec<_>>();
        for metadata in &metadata {
            let full_name = metadata.full_name();
            if !names.insert(type_short_name(full_name)) {
                return Err(build_error(format!("Rust 类型重复: {full_name}")));
            }
            match metadata.rust.definition_source() {
                Some(definition) => sections.push(definition.trim().to_string()),
                None => sections.push(format!("pub use {full_name};")),
            }
        }
        Ok(RustBuildResult {
            code: join_sections(sections),
            dependencies: dependencies(&metadata),
        })
    }
}

impl Default for RustTypeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 依据派生项与属性文本推断生成 crate 需要声明的依赖。
fn dependencies(metadata: &[TypeMetadata]) -> Vec<RustDependency> {
    let mut serde = false;
    let mut serde_with = false;
    for metadata in metadata {
        for derive in metadata.rust.derives {
            serde |= matches!(
                derive.rsplit("::").next(),
                Some("Serialize" | "Deserialize")
            );
        }
        for attribute in metadata.rust.attributes {
            serde_with |= attribute.contains("serde_as") || attribute.contains("serde_with::");
        }
    }
    let mut dependencies = Vec::new();
    if serde {
        dependencies.push(RustDependency::new("serde", "1").feature("derive"));
    }
    if serde_with {
        dependencies.push(RustDependency::new("serde_with", "3"));
    }
    dependencies
}
