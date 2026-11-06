use std::borrow::Borrow;
use std::collections::BTreeSet;

use crate::utils::{shouty_snake_case, snake_case};
use crate::{EnumMetadata, EnumValue};

use super::common::{
    RustBuildResult, RustBuilder, RustDependency, RustResult, build_error, join_sections,
    raw_ident, string_literal,
};
use super::utils::type_short_name;

/// 构建枚举运行时导出模块。
pub struct RustEnumBuilder {
    metadata: Vec<EnumMetadata>,
    type_import_from: Option<String>,
}

impl RustEnumBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            type_import_from: None,
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

    pub fn push_enums<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    /// 设置强类型枚举的来源模块；设置后以全路径限定追加强类型辅助函数。
    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from = Some(source.into());
        self
    }
}

impl RustBuilder for RustEnumBuilder {
    fn build(&self) -> RustResult<RustBuildResult> {
        let mut metadata = self.metadata.clone();
        metadata.sort_by_key(|metadata| metadata.full_name());

        let mut names = BTreeSet::new();
        let mut sections = Vec::new();
        let typed = self.type_import_from.as_deref();
        for metadata in &metadata {
            let name = type_short_name(metadata.full_name());
            if !names.insert(name.clone()) {
                return Err(build_error(format!("Rust 枚举名重复: {name}")));
            }
            let values = metadata.values()?;
            sections.push(declaration(metadata, &name, &values));
            if let Some(source) = typed {
                sections.push(helpers(source, &name));
            }
        }
        Ok(RustBuildResult {
            code: join_sections(sections),
            dependencies: if typed.is_some() {
                vec![RustDependency::new("serde_json", "1")]
            } else {
                Vec::new()
            },
        })
    }
}

impl Default for RustEnumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn declaration(metadata: &EnumMetadata, name: &str, values: &[EnumValue]) -> String {
    let shouty = shouty_snake_case(name);
    let fields = metadata
        .fields
        .iter()
        .map(|field| {
            format!(
                "    pub {}: {},\n",
                raw_ident(field),
                field_type(values, field)
            )
        })
        .collect::<String>();
    let all = values
        .iter()
        .map(|value| string_literal(&value.value))
        .collect::<Vec<_>>()
        .join(", ");
    let entries = values
        .iter()
        .map(|value| {
            let fields = metadata
                .fields
                .iter()
                .map(|field| {
                    let json = value
                        .fields
                        .iter()
                        .find(|(name, _)| *name == *field)
                        .map(|(_, json)| json);
                    format!(
                        "{}: {}",
                        raw_ident(field),
                        field_literal(json, &field_type(values, field)),
                    )
                })
                .collect::<Vec<_>>();
            let fields = if fields.is_empty() {
                String::new()
            } else {
                format!(", {}", fields.join(", "))
            };
            let value = string_literal(&value.value);
            format!("\n    ({value}, {name} {{ value: {value}{fields} }}),")
        })
        .collect::<String>();
    let entries = if entries.is_empty() {
        String::new()
    } else {
        format!("{entries}\n")
    };
    format!(
        "/// `{name}` 的运行时信息。\npub struct {name} {{\n    pub value: &'static str,\n{fields}}}\n\n/// `{name}` 的全部枚举值。\npub const {shouty}_ALL: &[&str] = &[{all}];\n\n/// `{name}` 枚举值到运行时信息的映射。\npub const {shouty}_MAP: &[(&str, {name})] = &[{entries}];"
    )
}

fn helpers(source: &str, name: &str) -> String {
    let prefix = snake_case(name);
    let shouty = shouty_snake_case(name);
    let typed = format!("{source}::{name}");
    format!(
        r#"/// 把 `{name}` 的字符串值转换为强类型枚举。
pub fn {prefix}_from_value(value: &str) -> Option<{typed}> {{
    serde_json::from_value(serde_json::Value::String(value.to_string())).ok()
}}

/// 返回 `{name}` 的全部强类型枚举值。
pub fn {prefix}_all() -> Vec<{typed}> {{
    {shouty}_ALL
        .iter()
        .filter_map(|value| {prefix}_from_value(value))
        .collect()
}}

/// 返回 `{name}` 强类型值与运行时信息的映射。
pub fn {prefix}_map() -> Vec<({typed}, &'static {name})> {{
    {shouty}_MAP
        .iter()
        .filter_map(|(value, info)| {prefix}_from_value(value).map(|value| (value, info)))
        .collect()
}}

/// 按 `{name}` 的字符串值查询运行时信息。
pub fn {prefix}_info(value: &str) -> Option<&'static {name}> {{
    {shouty}_MAP
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(_, info)| info)
}}"#
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Null,
    Bool,
    Integer,
    Float,
    Str,
    Other,
}

fn field_kind(value: &serde_json::Value) -> FieldKind {
    match value {
        serde_json::Value::Null => FieldKind::Null,
        serde_json::Value::Bool(_) => FieldKind::Bool,
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => {
            FieldKind::Integer
        }
        serde_json::Value::Number(_) => FieldKind::Float,
        serde_json::Value::String(_) => FieldKind::Str,
        _ => FieldKind::Other,
    }
}

/// 依据所有枚举值中该字段的 JSON 形态推断 Rust 类型。
fn field_type(values: &[EnumValue], field: &str) -> String {
    let mut kinds = Vec::new();
    for value in values {
        let Some((_, json)) = value.fields.iter().find(|(name, _)| *name == field) else {
            continue;
        };
        let kind = field_kind(json);
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    let optional = kinds.contains(&FieldKind::Null);
    let kinds = kinds
        .into_iter()
        .filter(|kind| *kind != FieldKind::Null)
        .collect::<Vec<_>>();
    let inner = match kinds.as_slice() {
        [] | [FieldKind::Str] => "&'static str",
        [FieldKind::Bool] => "bool",
        [FieldKind::Integer] => "i64",
        [FieldKind::Float] => "f64",
        [FieldKind::Integer, FieldKind::Float] | [FieldKind::Float, FieldKind::Integer] => "f64",
        _ => "&'static str",
    };
    if optional {
        format!("Option<{inner}>")
    } else {
        inner.to_string()
    }
}

fn field_literal(value: Option<&serde_json::Value>, ty: &str) -> String {
    let Some(inner) = ty
        .strip_prefix("Option<")
        .and_then(|ty| ty.strip_suffix('>'))
    else {
        return value.map_or_else(|| default_literal(ty), |value| plain_literal(value, ty));
    };
    match value {
        Some(value) if !value.is_null() => format!("Some({})", plain_literal(value, inner)),
        _ => "None".to_string(),
    }
}

fn plain_literal(value: &serde_json::Value, ty: &str) -> String {
    match ty {
        "bool" => value
            .as_bool()
            .map_or_else(|| "false".to_string(), |value| value.to_string()),
        "i64" => value
            .as_i64()
            .map_or_else(|| "0".to_string(), |value| value.to_string()),
        "f64" => value.as_f64().map_or_else(
            || "0.0".to_string(),
            |value| {
                if value.fract() == 0.0 {
                    format!("{value:.1}")
                } else {
                    format!("{value}")
                }
            },
        ),
        _ => match value {
            serde_json::Value::String(text) => string_literal(text),
            other => string_literal(&other.to_string()),
        },
    }
}

fn default_literal(ty: &str) -> String {
    if ty.starts_with("Option<") {
        return "None".to_string();
    }
    match ty {
        "bool" => "false".to_string(),
        "i64" => "0".to_string(),
        "f64" => "0.0".to_string(),
        _ => string_literal(""),
    }
}
