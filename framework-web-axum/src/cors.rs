use framework_core::MultiStringValue;
use std::sync::LazyLock;

static FIXED: LazyLock<AxumCors> = LazyLock::new(AxumCors::default);

/// CORS 配置，字段为各响应头的简短映射，`None` 表示不输出该响应头。
#[derive(Debug, Clone, Default)]
pub struct AxumCors {
    /// access-control-allow-origin
    pub origin: Option<String>,
    /// access-control-allow-methods
    pub methods: Option<String>,
    /// access-control-allow-headers
    pub headers: Option<String>,
    /// access-control-expose-headers
    pub expose: Option<String>,
    /// access-control-allow-credentials
    pub credentials: Option<String>,
    /// access-control-max-age
    pub max_age: Option<String>,
}

impl AxumCors {
    /// 懒加载的固定值：不输出任何 CORS 响应头，即拒绝跨域。
    pub fn fixed() -> &'static Self {
        &FIXED
    }

    /// 将配置写入响应头。
    pub fn apply(&self, headers: &mut MultiStringValue) {
        let items = [
            ("access-control-allow-origin", &self.origin),
            ("access-control-allow-methods", &self.methods),
            ("access-control-allow-headers", &self.headers),
            ("access-control-expose-headers", &self.expose),
            ("access-control-allow-credentials", &self.credentials),
            ("access-control-max-age", &self.max_age),
        ];
        for (name, value) in items {
            if let Some(value) = value {
                headers.set(name, value);
            }
        }
    }
}
