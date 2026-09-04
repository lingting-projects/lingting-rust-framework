use std::borrow::Borrow;
use std::collections::HashMap;

use crate::ApiMetadata;

use super::common::{build_error, is_typescript_identifier};
use super::utils::{camel_case, collect_api_type_names};
use super::{
    TypescriptBuildResult, TypescriptBuilder, TypescriptResult, api_class, api_definition,
};

/// 构建 TypeScript API 定义及可选的抽象类。
pub struct TypescriptApiBuilder {
    metadata: Vec<ApiMetadata>,
    class_name: Option<String>,
    class_enabled: bool,
    type_import_from: String,
}

impl TypescriptApiBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            class_name: None,
            class_enabled: true,
            type_import_from: ".".to_string(),
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

    pub fn class_name(mut self, name: impl Into<String>) -> Self {
        self.class_name = Some(name.into());
        self
    }

    /// 设置是否导出 API 抽象类，默认导出。
    pub fn with_class(mut self, enabled: bool) -> Self {
        self.class_enabled = enabled;
        self
    }

    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from = source.into();
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
}

impl TypescriptBuilder for TypescriptApiBuilder {
    fn build(&self) -> TypescriptResult<TypescriptBuildResult> {
        validate_api_metadata(&self.metadata)?;
        let type_import = type_import(&self.metadata, &self.type_import_from)?;
        let class_name = self
            .class_enabled
            .then(|| self.required_class_name())
            .transpose()?;
        let declaration_class = class_name
            .map(|name| api_class::declaration(name, &self.metadata))
            .unwrap_or_default();
        let javascript_class = class_name
            .map(|name| api_class::javascript(name, &self.metadata))
            .unwrap_or_default();
        Ok(TypescriptBuildResult {
            js: format!(
                "{}{}",
                api_definition::javascript(&self.metadata),
                javascript_class
            ),
            dts: format!(
                "{}{}{}",
                type_import,
                api_definition::declaration(&self.metadata),
                declaration_class,
            ),
        })
    }
}

impl Default for TypescriptApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypescriptApiBuilder {
    fn required_class_name(&self) -> TypescriptResult<&str> {
        let Some(name) = self.class_name.as_deref() else {
            return Err(build_error("缺少 TypeScript API 抽象类名"));
        };
        if !is_typescript_identifier(name) {
            return Err(build_error("TypeScript API 抽象类名必须是有效标识符"));
        }
        Ok(name)
    }
}

fn type_import(apis: &[ApiMetadata], source: &str) -> TypescriptResult<String> {
    if source.is_empty() {
        return Err(build_error("TypeScript API 类型导入来源不能为空"));
    }
    let names = collect_api_type_names(apis);
    if names.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "import type {{ {} }} from \"{}\";\n\n",
        names.into_iter().collect::<Vec<_>>().join(", "),
        source,
    ))
}

fn validate_api_metadata(apis: &[ApiMetadata]) -> TypescriptResult<()> {
    let mut names = HashMap::<String, Vec<&ApiMetadata>>::new();
    let mut routes = HashMap::<String, Vec<&ApiMetadata>>::new();
    for api in apis {
        names.entry(camel_case(api.name)).or_default().push(api);
        routes
            .entry(format!("{} {}", api.method, api.path))
            .or_default()
            .push(api);
    }
    let mut problems = Vec::new();
    for (name, duplicates) in names.into_iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "方法名重复 {name}: {}",
            duplicate_sources(&duplicates)
        ));
    }
    for (route, duplicates) in routes.into_iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "请求方法和地址重复 {route}: {}",
            duplicate_sources(&duplicates)
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        problems.sort();
        Err(build_error(format!(
            "TypeScript API 导出失败:\n{}",
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
