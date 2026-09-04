use std::borrow::Borrow;
use std::collections::BTreeSet;

use crate::{EnumMetadata, EnumValue};

use super::common::{javascript_property, json_string, type_short_name, typescript_property};
use super::{TypescriptBuildResult, TypescriptBuilder, TypescriptResult};

/// 构建枚举运行时导出。
pub struct TypescriptEnumBuilder {
    metadata: Vec<EnumMetadata>,
    type_import_from: String,
}

impl TypescriptEnumBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            type_import_from: ".".to_string(),
        }
    }

    pub fn enums<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.push_enums(metadata);
        self
    }

    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from = source.into();
        self
    }

    pub fn push_enums<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }
}

impl TypescriptBuilder for TypescriptEnumBuilder {
    fn build(&self) -> TypescriptResult<TypescriptBuildResult> {
        let names = self
            .metadata
            .iter()
            .map(|metadata| type_short_name(metadata.full_name()))
            .collect::<BTreeSet<_>>();
        let dts = format!(
            "{}{}",
            type_import(&names, &self.type_import_from)?,
            declaration(&self.metadata)?
        );
        Ok(TypescriptBuildResult {
            js: javascript(&self.metadata)?,
            dts,
        })
    }
}

impl Default for TypescriptEnumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn type_import(names: &BTreeSet<String>, source: &str) -> TypescriptResult<String> {
    if source.is_empty() {
        return Err(super::common::build_error(
            "TypeScript 类型导入来源不能为空",
        ));
    }
    if names.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "import type {{ {} }} from \"{}\";\n\n",
        names.iter().cloned().collect::<Vec<_>>().join(", "),
        source,
    ))
}

fn declaration(enums: &[EnumMetadata]) -> TypescriptResult<String> {
    let mut result = String::new();
    let mut names = BTreeSet::new();
    for metadata in enums {
        let name = type_short_name(metadata.full_name());
        if !names.insert(name.clone()) {
            return Err(super::common::build_error(format!(
                "枚举运行时导出重复: {name}"
            )));
        }
        let values = metadata.values()?;
        let fields = metadata
            .fields
            .iter()
            .map(|field| {
                let types = values
                    .iter()
                    .filter_map(|value| {
                        value
                            .fields
                            .iter()
                            .find(|(name, _)| name == field)
                            .map(|(_, value)| typescript_value_type(value))
                    })
                    .collect::<BTreeSet<_>>();
                let type_name = if types.is_empty() {
                    "unknown".to_string()
                } else {
                    types.into_iter().collect::<Vec<_>>().join(" | ")
                };
                format!("{}: {type_name};", typescript_property(field))
            })
            .collect::<Vec<_>>()
            .join(" ");
        let record = if fields.is_empty() {
            format!("{{ value: {name}; }}")
        } else {
            format!("{{ value: {name}; {fields} }}")
        };
        result.push_str(&format!(
            "export declare const {name}All: readonly {name}[];\nexport declare const {name}Map: Readonly<Record<{name}, {record}>>;\n"
        ));
    }
    Ok(result)
}

fn javascript(enums: &[EnumMetadata]) -> TypescriptResult<String> {
    let mut result = String::new();
    for metadata in enums {
        let name = type_short_name(metadata.full_name());
        let values = metadata.values()?;
        let all = values
            .iter()
            .map(|value| json_string(&value.value))
            .collect::<TypescriptResult<Vec<_>>>()?
            .join(", ");
        let map = values
            .iter()
            .map(javascript_enum_value)
            .collect::<TypescriptResult<Vec<_>>>()?
            .join(", ");
        result.push_str(&format!(
            "export const {name}All = [{all}];\nexport const {name}Map = {{ {map} }};\n\n"
        ));
    }
    Ok(result)
}

fn javascript_enum_value(value: &EnumValue) -> TypescriptResult<String> {
    let mut properties = vec![format!("value: {}", json_string(&value.value)?)];
    for (field, field_value) in &value.fields {
        properties.push(format!(
            "{}: {}",
            javascript_property(field),
            serde_json::to_string(field_value)?,
        ));
    }
    Ok(format!(
        "{}: {{ {} }}",
        json_string(&value.value)?,
        properties.join(", "),
    ))
}

fn typescript_value_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "unknown[]",
        serde_json::Value::Object(_) => "Record<string, unknown>",
    }
}
