//! 自启级别：系统级或当前用户级。

/// 自启的注册级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// 系统级：随系统启动，不依赖用户登录；注册与启停通常需要管理员或 root 权限。
    Root,
    /// 用户级：随当前用户登录启动。
    User,
}
