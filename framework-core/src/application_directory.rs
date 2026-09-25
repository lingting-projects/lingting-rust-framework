use crate::{home_directory, system_directory};
use anyhow::{Context, Result};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// 应用运行所需的目录集合。
#[derive(Debug)]
pub struct ApplicationDirectory {
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
        if cfg!(debug_assertions) {
            return Self::with_debug();
        }

        let id = id.as_ref();
        let install = install_directory()?;
        let root = system_directory()?.join(id);
        let temp = env::temp_dir();

        Self::new(
            create_directory(&root.join("data"))?,
            create_directory(&root.join("cache"))?,
            create_directory(&temp.join(id))?,
            create_directory(&temp.join(format!("{id}_logs")))?,
            install,
        )
    }

    /// 创建使用当前用户目录的应用目录。
    pub fn user(id: impl AsRef<str>) -> Result<Self> {
        if cfg!(debug_assertions) {
            return Self::with_debug();
        }

        let id = id.as_ref();
        let install = install_directory()?;
        let home = home_directory()?;
        let username = home
            .file_name()
            .map(OsStr::to_string_lossy)
            .with_context(|| format!("无法从用户目录获取用户名: {}", home.display()))?
            .into_owned();
        let root = home.join(id);
        let temp = env::temp_dir();

        Self::new(
            create_directory(&root.join("data"))?,
            create_directory(&root.join("cache"))?,
            create_directory(&temp.join(format!("{id}_{username}")))?,
            create_directory(&temp.join(format!("{id}_{username}_logs")))?,
            install,
        )
    }

    /// 创建使用指定根目录的应用目录。
    pub fn with(root: impl AsRef<Path>) -> Result<Self> {
        if cfg!(debug_assertions) {
            return Self::with_debug();
        }

        Self::wrapper(root.as_ref())
    }

    /// 调试模式统一使用运行目录。
    fn with_debug() -> Result<Self> {
        let install = install_directory()?;
        Self::wrapper(&debug_runtime_directory(&install))
    }

    /// 使用指定目录作为根目录。
    fn wrapper(root: &Path) -> Result<Self> {
        let install = install_directory()?;

        Self::new(
            create_directory(&root.join("data"))?,
            create_directory(&root.join("cache"))?,
            create_directory(&root.join("tmp"))?,
            create_directory(&root.join("logs"))?,
            install,
        )
    }

    fn new(
        data: PathBuf,
        cache: PathBuf,
        tmp: PathBuf,
        logs: PathBuf,
        install: PathBuf,
    ) -> Result<Self> {
        let startup = env::current_dir().context("获取启动目录失败")?;

        Ok(Self {
            data,
            cache,
            tmp,
            logs,
            startup,
            install,
        })
    }
}

fn create_directory(directory: &Path) -> Result<PathBuf> {
    fs::create_dir_all(directory)
        .with_context(|| format!("创建应用目录失败: {}", directory.display()))?;
    Ok(directory.to_path_buf())
}

fn install_directory() -> Result<PathBuf> {
    let executable = env::current_exe().context("获取当前可执行文件失败")?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .context("未找到当前可执行文件所在目录")
}

fn debug_runtime_directory(install: &Path) -> PathBuf {
    let target = install
        .ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == "target"));

    if let Some(target) = target
        && let Ok(relative) = install.strip_prefix(target)
    {
        let mut parts: Vec<&OsStr> = relative.components().map(|part| part.as_os_str()).collect();

        if parts.pop() == Some(OsStr::new("bin")) {
            if parts.last().copied() == Some(OsStr::new("examples")) {
                parts.pop();
            }
            if matches!(parts.len(), 1 | 2) {
                return target.join("runtime");
            }
        }
    }

    install.join("runtime")
}
