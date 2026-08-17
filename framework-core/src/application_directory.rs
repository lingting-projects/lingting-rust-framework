use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// 应用运行所需的目录集合。
#[derive(Debug)]
pub struct ApplicationDirectory {
    pub id: String,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub tmp: PathBuf,
    pub logs: PathBuf,
    pub startup: PathBuf,
    pub install: PathBuf,
}

impl ApplicationDirectory {
    /// 创建使用系统公共目录的应用目录。
    pub fn root(id: impl AsRef<str>) -> Result<Self> {
        let install = install_directory()?;
        let global = if cfg!(debug_assertions) {
            debug_runtime_directory(&install)
        } else {
            system_directory()?.join(id.as_ref())
        };
        Self::new(id, global, install)
    }

    /// 创建使用当前用户目录的应用目录。
    pub fn normal(id: impl AsRef<str>, parent: impl AsRef<str>) -> Result<Self> {
        let install = install_directory()?;
        let global = if cfg!(debug_assertions) {
            debug_runtime_directory(&install)
        } else {
            user_directory()?
        };
        let global = global.join(parent.as_ref()).join(id.as_ref());
        Self::new(id, global, install)
    }

    fn new(id: impl AsRef<str>, global: PathBuf, install: PathBuf) -> Result<Self> {
        fs::create_dir_all(&global)
            .with_context(|| format!("创建应用目录失败: {}", global.display()))?;

        let data = create_directory(&global, "data")?;
        let cache = create_directory(&global, "cache")?;
        let tmp = create_directory(&global, "tmp")?;
        let logs = create_directory(&tmp, "logs")?;
        let startup = env::current_dir().context("获取启动目录失败")?;

        Ok(Self {
            id: id.as_ref().to_owned(),
            data,
            cache,
            tmp,
            logs,
            startup,
            install,
        })
    }
}

fn create_directory(parent: &Path, name: &str) -> Result<PathBuf> {
    let directory = parent.join(name);
    fs::create_dir_all(&directory)
        .with_context(|| format!("创建应用目录失败: {}", directory.display()))?;
    Ok(directory)
}

fn install_directory() -> Result<PathBuf> {
    let executable = env::current_exe().context("获取当前可执行文件失败")?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .context("未找到当前可执行文件所在目录")
}

fn debug_runtime_directory(install: &Path) -> PathBuf {
    let profile = install.parent();
    let is_example = install.file_name().is_some_and(|name| name == "examples")
        && profile.is_some_and(|path| {
        path.ancestors()
            .skip(1)
            .any(|ancestor| ancestor.file_name().is_some_and(|name| name == "target"))
    });
    let base = is_example.then_some(profile.unwrap()).unwrap_or(install);
    base.join("runtime")
}

fn system_directory() -> Result<PathBuf> {
    if cfg!(windows) {
        return env::var_os("ALLUSERSPROFILE")
            .map(PathBuf::from)
            .context("未设置 ALLUSERSPROFILE 环境变量");
    }
    if cfg!(target_os = "linux") {
        return Ok(PathBuf::from("/usr/local/share"));
    }
    Ok(PathBuf::from("/Library/Application Support"))
}

fn user_directory() -> Result<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    env::var_os(variable)
        .map(PathBuf::from)
        .with_context(|| format!("未设置 {variable} 环境变量"))
}
