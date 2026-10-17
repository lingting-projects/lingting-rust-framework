//! 扫描源码目录中的 `#[timer]` 任务并生成名称枚举代码。
//!
//! 生成的枚举实现了 [`crate::TimerNameLike`]，因此既能传给 [`crate::wake`]，
//! 也能直接用于日志等需要名称的场景。返回值是完整的 Rust 源码字符串，
//! 由调用方决定写入文件还是直接使用。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::Visit;

/// 定时任务名称枚举的默认类型名。
pub const DEFAULT_ENUM_NAME: &str = "TimerName";

struct TimerTask {
    name: String,
    path: PathBuf,
}

#[derive(Default)]
struct TimerVisitor {
    tasks: Vec<String>,
}

impl<'ast> Visit<'ast> for TimerVisitor {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if item.attrs.iter().any(is_timer_attribute) {
            self.tasks.push(item.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, item);
    }
}

/// 扫描 `directory` 下全部 Rust 源文件中的 `#[timer]` 任务，生成名称枚举代码。
///
/// 枚举变体由任务函数名转换为大驼峰，变体名冲突或任务重名时返回错误。
pub fn generate(directory: impl AsRef<Path>) -> Result<String, String> {
    generate_named(directory, DEFAULT_ENUM_NAME)
}

/// 与 [`generate`] 相同，但使用 `enum_name` 作为生成的枚举类型名。
pub fn generate_named(directory: impl AsRef<Path>, enum_name: &str) -> Result<String, String> {
    let directory = directory.as_ref();
    if !directory.is_dir() {
        return Err(format!("{} 不是有效目录", directory.display()));
    }
    syn::parse_str::<syn::Ident>(enum_name)
        .map_err(|_| format!("枚举名 `{enum_name}` 不是合法的 Rust 标识符"))?;

    let mut tasks = Vec::new();
    collect_timer_tasks(directory, &mut tasks)?;
    let variants = build_variants(tasks)?;
    Ok(render_timer_name(enum_name, &variants))
}

fn collect_timer_tasks(directory: &Path, tasks: &mut Vec<TimerTask>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("读取目录 {} 失败：{error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_timer_tasks(&path, tasks)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            collect_timer_tasks_from_file(&path, tasks)?;
        }
    }
    Ok(())
}

fn collect_timer_tasks_from_file(path: &Path, tasks: &mut Vec<TimerTask>) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("解析 {} 失败：{error}", path.display()))?;
    let mut visitor = TimerVisitor::default();
    visitor.visit_file(&syntax);
    tasks.extend(visitor.tasks.into_iter().map(|name| TimerTask {
        name,
        path: path.to_owned(),
    }));
    Ok(())
}

fn is_timer_attribute(attribute: &syn::Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "timer")
}

fn build_variants(mut tasks: Vec<TimerTask>) -> Result<Vec<(String, TimerTask)>, String> {
    let mut names = BTreeMap::new();
    let mut variants = BTreeMap::new();
    for task in &tasks {
        if let Some(previous) = names.insert(task.name.clone(), task.path.clone()) {
            return Err(format!(
                "重复的定时任务名 `{}`：{} 与 {}",
                task.name,
                previous.display(),
                task.path.display()
            ));
        }

        let variant = to_pascal_case(&task.name)?;
        if let Some(previous) = variants.insert(variant.clone(), task) {
            return Err(format!(
                "定时任务名称枚举变体 `{variant}` 冲突：任务 `{}`（{}）与 `{}`（{}）",
                previous.name,
                previous.path.display(),
                task.name,
                task.path.display()
            ));
        }
    }
    tasks.sort_by(|left, right| left.name.cmp(&right.name));
    tasks
        .into_iter()
        .map(|task| Ok((to_pascal_case(&task.name)?, task)))
        .collect()
}

fn to_pascal_case(name: &str) -> Result<String, String> {
    let variant = name
        .strip_prefix("r#")
        .unwrap_or(name)
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };
            first.to_uppercase().chain(characters).collect::<String>()
        })
        .collect::<String>();
    syn::parse_str::<syn::Ident>(&variant)
        .map_err(|_| format!("任务名 `{name}` 无法生成合法的枚举变体 `{variant}`"))?;
    Ok(variant)
}

fn render_timer_name(enum_name: &str, variants: &[(String, TimerTask)]) -> String {
    let variant_definitions = variants
        .iter()
        .map(|(variant, _)| format!("    {variant},"))
        .collect::<Vec<_>>()
        .join("\n");
    let names = variants
        .iter()
        .map(|(variant, task)| format!("            Self::{variant} => \"{}\",", task.name))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"/// 由源码目录中的 `#[timer]` 任务生成的名称枚举。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum {enum_name} {{
{variant_definitions}
}}

impl {enum_name} {{
    /// 返回与 `TimerRegistration::name` 一致的原始任务函数名。
    pub const fn as_str(self) -> &'static str {{
        match self {{
{names}
        }}
    }}
}}

impl ::framework_timer::TimerNameLike for {enum_name} {{
    fn timer_name(&self) -> &str {{
        self.as_str()
    }}
}}

impl std::fmt::Display for {enum_name} {{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        formatter.write_str(self.as_str())
    }}
}}
"#
    )
}
