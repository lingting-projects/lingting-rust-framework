use std::fmt::{Display, Formatter};

/// 支持的 HTTP 方法。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// 读取资源
    Get,
    /// 提交数据
    Post,
    /// 整体替换资源
    Put,
    /// 局部更新资源
    Patch,
    /// 删除资源
    Delete,
    /// 只取响应头
    Head,
    /// 查询资源支持的方法
    Options,
}

impl HttpMethod {
    /// 协议中书写的方法名，全大写。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
