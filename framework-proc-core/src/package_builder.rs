use std::borrow::Borrow;
use std::fs;
use std::path::PathBuf;

use crate::{
    ApiMetadata, EnumMetadata, GeneratedTypeScript, TypeMetadata, TypeScriptCodeBuilder,
    TypeScriptResult,
};

/// 将 TypeScript 内容写入 npm ESM 包目录。
pub struct TypeScriptPackageBuilder {
    package_name: String,
    version: String,
    output_dir: PathBuf,
    code_builder: TypeScriptCodeBuilder,
}

impl TypeScriptPackageBuilder {
    pub fn new(
        package_name: impl Into<String>,
        version: impl Into<String>,
        output_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            version: version.into(),
            output_dir: output_dir.into(),
            code_builder: TypeScriptCodeBuilder::new(),
        }
    }

    pub fn types<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.code_builder.push_types(metadata);
        self
    }

    pub fn apis<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.code_builder.push_apis(metadata);
        self
    }

    pub fn enums<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.code_builder.push_enums(metadata);
        self
    }

    pub fn api_class_name(mut self, name: impl Into<String>) -> Self {
        self.code_builder = std::mem::take(&mut self.code_builder).api_class_name(name);
        self
    }

    pub fn api_type_import_from(mut self, package_name: impl Into<String>) -> Self {
        self.code_builder =
            std::mem::take(&mut self.code_builder).api_type_import_from(package_name);
        self
    }

    pub fn build(&self) -> TypeScriptResult<GeneratedTypeScript> {
        self.code_builder.build()
    }

    pub fn write(&self) -> TypeScriptResult<()> {
        let generated = self.code_builder.build()?;
        let dist_dir = self.output_dir.join("dist");

        fs::create_dir_all(&dist_dir)?;
        fs::write(dist_dir.join("index.js"), generated.javascript)?;
        fs::write(dist_dir.join("index.d.ts"), generated.declaration)?;
        fs::write(
            self.output_dir.join("package.json"),
            package_json(&self.package_name, &self.version)?,
        )?;
        Ok(())
    }
}

fn package_json(package_name: &str, version: &str) -> TypeScriptResult<String> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "name": package_name,
            "version": version,
            "type": "module",
            "main": "./dist/index.js",
            "types": "./dist/index.d.ts",
            "exports": {
                ".": {
                    "types": "./dist/index.d.ts",
                    "default": "./dist/index.js"
                }
            }
        }))?
    ))
}
