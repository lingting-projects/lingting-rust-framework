use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::path::PathBuf;

use super::common::{
    RustBuildResult, RustBuilder, RustDependency, RustResult, build_error, is_rust_identifier,
    string_literal,
};

/// 把多个构建器结果写入一个 Rust crate 目录。
pub struct CrateBuilder {
    crate_name: String,
    version: String,
    edition: String,
    rust_version: Option<String>,
    output_dir: PathBuf,
    dependencies: Vec<RustDependency>,
    builders: Vec<(String, Box<dyn RustBuilder>)>,
}

impl CrateBuilder {
    pub fn new(
        crate_name: impl Into<String>,
        version: impl Into<String>,
        output_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            crate_name: crate_name.into(),
            version: version.into(),
            edition: "2024".to_string(),
            rust_version: None,
            output_dir: output_dir.into(),
            dependencies: Vec::new(),
            builders: Vec::new(),
        }
    }

    pub fn edition(mut self, edition: impl Into<String>) -> Self {
        self.edition = edition.into();
        self
    }

    pub fn rust_version(mut self, version: impl Into<String>) -> Self {
        self.rust_version = Some(version.into());
        self
    }

    /// 补充构建器无法推断的第三方依赖。
    pub fn dependency(&mut self, dependency: RustDependency) {
        self.dependencies.push(dependency);
    }

    pub fn push(&mut self, module: impl Into<String>, builder: impl RustBuilder + 'static) {
        self.builders.push((module.into(), Box::new(builder)));
    }

    pub fn write(&self) -> RustResult<()> {
        if !is_crate_name(&self.crate_name) {
            return Err(build_error(format!("无效的 crate 名: {}", self.crate_name)));
        }
        if self.version.trim().is_empty() {
            return Err(build_error("crate 版本不能为空"));
        }
        validate_modules(&self.builders)?;
        let mut built = Vec::new();
        for (name, builder) in &self.builders {
            built.push((name, builder.build()?));
        }
        let dependencies = self.merged_dependencies(&built)?;

        let source_dir = self.output_dir.join("src");
        fs::create_dir_all(&source_dir)?;
        for (name, result) in &built {
            fs::write(source_dir.join(format!("{name}.rs")), &result.code)?;
        }
        fs::write(source_dir.join("lib.rs"), library(&self.builders))?;
        fs::write(
            self.output_dir.join("Cargo.toml"),
            self.cargo_toml(&dependencies),
        )?;
        Ok(())
    }

    fn merged_dependencies(
        &self,
        built: &[(&String, RustBuildResult)],
    ) -> RustResult<Vec<RustDependency>> {
        let mut merged = BTreeMap::<String, RustDependency>::new();
        let dependencies = self
            .dependencies
            .iter()
            .chain(built.iter().flat_map(|(_, result)| &result.dependencies));
        for dependency in dependencies {
            match merged.entry(dependency.name.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(dependency.clone());
                }
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut();
                    if existing.version != dependency.version {
                        return Err(build_error(format!(
                            "依赖版本冲突 {}: {} != {}",
                            dependency.name, existing.version, dependency.version,
                        )));
                    }
                    for feature in &dependency.features {
                        if !existing.features.contains(feature) {
                            existing.features.push(feature.clone());
                        }
                    }
                }
            }
        }
        Ok(merged.into_values().collect())
    }

    fn cargo_toml(&self, dependencies: &[RustDependency]) -> String {
        let mut lines = vec![
            "[package]".to_string(),
            format!("name = {}", string_literal(&self.crate_name)),
            format!("version = {}", string_literal(&self.version)),
            format!("edition = {}", string_literal(&self.edition)),
        ];
        if let Some(rust_version) = &self.rust_version {
            lines.push(format!("rust-version = {}", string_literal(rust_version)));
        }
        lines.push(String::new());
        lines.push("[dependencies]".to_string());
        lines.extend(dependencies.iter().map(dependency_line));
        format!("{}\n", lines.join("\n"))
    }
}

fn is_crate_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(value) if value.is_ascii_lowercase())
        && characters.all(|value| {
            value.is_ascii_lowercase() || value.is_ascii_digit() || value == '_' || value == '-'
        })
}

fn validate_modules(builders: &[(String, Box<dyn RustBuilder>)]) -> RustResult<()> {
    let mut names = std::collections::BTreeSet::new();
    for (name, _) in builders {
        if name == "lib" || name == "main" {
            return Err(build_error(format!("无效的模块名: {name}")));
        }
        if !is_rust_identifier(name) {
            return Err(build_error(format!("无效的模块名: {name}")));
        }
        if !names.insert(name) {
            return Err(build_error(format!("模块名重复: {name}")));
        }
    }
    Ok(())
}

fn library(builders: &[(String, Box<dyn RustBuilder>)]) -> String {
    let mut lines = builders
        .iter()
        .map(|(name, _)| format!("pub mod {name};"))
        .collect::<Vec<_>>();
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.extend(
        builders
            .iter()
            .map(|(name, _)| format!("pub use {name}::*;")),
    );
    format!("{}\n", lines.join("\n"))
}

fn dependency_line(dependency: &RustDependency) -> String {
    if dependency.features.is_empty() {
        return format!(
            "{} = {}",
            dependency.name,
            string_literal(&dependency.version)
        );
    }
    let features = dependency
        .features
        .iter()
        .map(|feature| string_literal(feature))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{} = {{ version = {}, features = [{}] }}",
        dependency.name,
        string_literal(&dependency.version),
        features,
    )
}
