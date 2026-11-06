use std::collections::BTreeSet;

/// 无需从其他模块导入的 Rust 类型名。
const BUILTIN_TYPES: &[&str] = &[
    "String", "str", "char", "bool", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
    "u32", "u64", "u128", "usize", "f32", "f64", "Option", "Vec", "HashMap", "BTreeMap", "Box",
    "Value", "Self",
];

/// 取完整类型名的最后一段，例如 `crate::dto::User` 得到 `User`。
pub(crate) fn type_short_name(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

/// 从类型文本中收集需要导入的类型名，忽略内置类型。
pub(crate) fn collect_type_names(value: &str, names: &mut BTreeSet<String>) {
    for name in
        value.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
    {
        if !name.is_empty() && !BUILTIN_TYPES.contains(&name) {
            names.insert(name.to_string());
        }
    }
}

/// 生成导入语句；没有需要导入的类型时返回 `None`。
pub(crate) fn import_line(source: &str, names: &BTreeSet<String>) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let names = names.iter().cloned().collect::<Vec<_>>();
    if names.len() == 1 {
        return Some(format!("use {source}::{};", names[0]));
    }
    Some(format!("use {source}::{{{}}};", names.join(", ")))
}
