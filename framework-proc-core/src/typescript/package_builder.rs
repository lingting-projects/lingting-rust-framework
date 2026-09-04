use std::fs;
use std::path::PathBuf;

use super::{TypescriptBuilder, TypescriptResult};

/// 将多个 TypeScript 模块写入 npm ESM 包目录。
pub struct PackageBuilder {
    package_name: String,
    version: String,
    output_dir: PathBuf,
    builders: Vec<(String, Box<dyn TypescriptBuilder>)>,
}

impl PackageBuilder {
    pub fn new(
        package_name: impl Into<String>,
        version: impl Into<String>,
        output_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            version: version.into(),
            output_dir: output_dir.into(),
            builders: Vec::new(),
        }
    }

    pub fn push(&mut self, name: impl Into<String>, builder: impl TypescriptBuilder + 'static) {
        self.builders.push((name.into(), Box::new(builder)));
    }

    pub fn write(&self) -> TypescriptResult<()> {
        validate_package(&self.package_name, &self.version)?;
        validate_module_names(&self.builders)?;
        let dist_dir = self.output_dir.join("dist");
        let generated = self
            .builders
            .iter()
            .map(|(name, builder)| Ok((name, builder.build()?)))
            .collect::<TypescriptResult<Vec<_>>>()?;

        fs::create_dir_all(&dist_dir)?;
        for (name, content) in generated {
            fs::write(dist_dir.join(format!("{name}.js")), content.js)?;
            fs::write(dist_dir.join(format!("{name}.d.ts")), content.dts)?;
        }
        fs::write(dist_dir.join("index.js"), index_js(&self.builders))?;
        fs::write(dist_dir.join("index.d.ts"), index_dts(&self.builders))?;
        fs::write(
            self.output_dir.join("package.json"),
            package_json(&self.package_name, &self.version, &self.builders)?,
        )?;
        Ok(())
    }
}

fn validate_package(package_name: &str, version: &str) -> TypescriptResult<()> {
    if package_name.is_empty() {
        return Err(super::common::build_error("package.json 的 name 不能为空"));
    }
    if version.is_empty() {
        return Err(super::common::build_error(
            "package.json 的 version 不能为空",
        ));
    }
    Ok(())
}

fn validate_module_names(
    builders: &[(String, Box<dyn TypescriptBuilder>)],
) -> TypescriptResult<()> {
    let mut names = std::collections::BTreeSet::new();
    for (name, _) in builders {
        if name.is_empty() || name == "index" || name.contains(['/', '\\']) {
            return Err(super::common::build_error(format!(
                "无效 TypeScript 模块名: {name}"
            )));
        }
        if !names.insert(name) {
            return Err(super::common::build_error(format!(
                "TypeScript 模块名重复: {name}"
            )));
        }
    }
    Ok(())
}

fn index_js(builders: &[(String, Box<dyn TypescriptBuilder>)]) -> String {
    builders
        .iter()
        .map(|(name, _)| format!("export * from \"./{name}.js\";\n"))
        .collect()
}

fn index_dts(builders: &[(String, Box<dyn TypescriptBuilder>)]) -> String {
    builders
        .iter()
        .map(|(name, _)| format!("export * from \"./{name}.js\";\n"))
        .collect()
}

fn package_json(
    package_name: &str,
    version: &str,
    builders: &[(String, Box<dyn TypescriptBuilder>)],
) -> TypescriptResult<String> {
    let mut exports = serde_json::Map::new();
    exports.insert(
        ".".to_string(),
        export_entry("./dist/index.js", "./dist/index.d.ts"),
    );
    for (name, _) in builders {
        exports.insert(
            format!("./{name}"),
            export_entry(&format!("./dist/{name}.js"), &format!("./dist/{name}.d.ts")),
        );
    }
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "name": package_name,
            "version": version,
            "type": "module",
            "main": "./dist/index.js",
            "types": "./dist/index.d.ts",
            "exports": exports,
        }))?
    ))
}

fn export_entry(js: &str, dts: &str) -> serde_json::Value {
    serde_json::json!({
        "types": dts,
        "default": js,
    })
}
