use anyhow::Context;
use std::env;
use std::path::PathBuf;

pub fn system_directory() -> anyhow::Result<PathBuf> {
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

pub fn home_directory() -> anyhow::Result<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    env::var_os(variable)
        .map(PathBuf::from)
        .with_context(|| format!("未设置 {variable} 环境变量"))
}
