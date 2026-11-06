use std::io;

/// Rust 代码生成结果。
pub type RustResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 单个模块的构建结果。
pub struct RustBuildResult {
    /// 模块文件内容（不含 `mod` 声明），写入 `src/<module>.rs`。
    pub code: String,
    /// 该模块需要的 Cargo 依赖，由 `CrateBuilder` 汇总去重。
    pub dependencies: Vec<RustDependency>,
}

/// 生成 crate 需要声明的 Cargo 依赖。
#[derive(Clone)]
pub struct RustDependency {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

impl RustDependency {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            features: Vec::new(),
        }
    }

    pub fn feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }
}

/// 构建一个 Rust 模块。
pub trait RustBuilder {
    fn build(&self) -> RustResult<RustBuildResult>;
}

pub(crate) fn build_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::other(message.into()))
}

/// 把各段落用空行连接，段落之间不再额外处理标点。
pub(crate) fn join_sections(sections: Vec<String>) -> String {
    let sections = sections
        .into_iter()
        .map(|section| section.trim().to_string())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        String::new()
    } else {
        format!("{}\n", sections.join("\n\n"))
    }
}

pub(crate) fn is_rust_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(value) if value.is_ascii_alphabetic() || value == '_')
        && characters.all(|value| value.is_ascii_alphanumeric() || value == '_')
        && !is_keyword(name)
}

/// 把 Rust 关键字转换为原始标识符，其余名称原样返回。
pub(crate) fn raw_ident(name: &str) -> String {
    if is_keyword(name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

pub(crate) fn string_literal(value: &str) -> String {
    format!("{value:?}")
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "union"
    )
}
