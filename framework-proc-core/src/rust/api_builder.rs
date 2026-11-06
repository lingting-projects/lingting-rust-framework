use std::borrow::Borrow;
use std::collections::BTreeMap;

use crate::ApiMetadata;
use crate::utils::snake_case;

use super::api_definition;
use super::api_trait;
use super::common::{
    RustBuildResult, RustBuilder, RustDependency, RustResult, build_error, is_rust_identifier,
    join_sections,
};
use super::utils::import_line;

/// 与 `ApiClient` 自身方法冲突的接口方法名。
const RESERVED_METHODS: &[&str] = &["send", "new", "call", "default", "from", "into"];

/// 构建 Rust 接口定义与客户端 trait。
pub struct RustApiBuilder {
    metadata: Vec<ApiMetadata>,
    trait_name: Option<String>,
    trait_enabled: bool,
    type_import_from: String,
}

impl RustApiBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            trait_name: None,
            trait_enabled: true,
            type_import_from: String::new(),
        }
    }

    pub fn apis<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.push_apis(metadata);
        self
    }

    pub fn push_apis<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    pub fn trait_name(mut self, name: impl Into<String>) -> Self {
        self.trait_name = Some(name.into());
        self
    }

    /// 设置是否导出客户端 trait，默认导出。
    pub fn with_trait(mut self, enabled: bool) -> Self {
        self.trait_enabled = enabled;
        self
    }

    /// 设置接口类型所在模块，用于生成 `use <source>::{...};`。
    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from = source.into();
        self
    }
}

impl RustBuilder for RustApiBuilder {
    fn build(&self) -> RustResult<RustBuildResult> {
        let mut metadata = self.metadata.clone();
        metadata.sort_by_key(|api| api.name);
        validate(&metadata)?;
        let trait_name = self.required_trait_name()?;
        let mut sections = Vec::new();
        if !self.type_import_from.is_empty()
            && let Some(line) = import_line(
                &self.type_import_from,
                &api_definition::type_names(&metadata),
            )
        {
            sections.push(line);
        }
        sections.push(api_definition::declaration(&metadata)?);
        if let Some(trait_name) = trait_name {
            sections.push(api_trait::declaration(trait_name, &metadata)?);
        }
        Ok(RustBuildResult {
            code: join_sections(sections),
            dependencies: vec![
                RustDependency::new("serde", "1"),
                RustDependency::new("serde_json", "1"),
            ],
        })
    }
}

impl Default for RustApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RustApiBuilder {
    fn required_trait_name(&self) -> RustResult<Option<&str>> {
        if !self.trait_enabled {
            return Ok(None);
        }
        let Some(name) = self.trait_name.as_deref() else {
            return Err(build_error("缺少 Rust API trait 名"));
        };
        if !is_rust_identifier(name) {
            return Err(build_error("Rust API trait 名必须是有效标识符"));
        }
        Ok(Some(name))
    }
}

fn validate(apis: &[ApiMetadata]) -> RustResult<()> {
    let mut names = BTreeMap::<String, Vec<&ApiMetadata>>::new();
    let mut routes = BTreeMap::<String, Vec<&ApiMetadata>>::new();
    let mut problems = Vec::new();
    for api in apis {
        let name = snake_case(api.name);
        if !is_rust_identifier(&name) {
            problems.push(format!(
                "方法名不是有效的 Rust 标识符 {}::{}: {name}",
                api.namespace, api.name
            ));
        }
        if RESERVED_METHODS.contains(&name.as_str()) {
            problems.push(format!(
                "方法名与 ApiClient 保留方法冲突 {}::{}: {name}",
                api.namespace, api.name
            ));
        }
        names.entry(name).or_default().push(api);
        routes
            .entry(format!("{} {}", api.method, api.path))
            .or_default()
            .push(api);
    }
    for (name, duplicates) in names.iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "方法名重复 {name}: {}",
            duplicate_sources(duplicates)
        ));
    }
    for (route, duplicates) in routes.iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "请求方法和地址重复 {route}: {}",
            duplicate_sources(duplicates)
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        problems.sort();
        Err(build_error(format!(
            "Rust API 导出失败:\n{}",
            problems.join("\n")
        )))
    }
}

fn duplicate_sources(apis: &[&ApiMetadata]) -> String {
    apis.iter()
        .map(|api| format!("{}::{}", api.namespace, api.name))
        .collect::<Vec<_>>()
        .join(", ")
}
