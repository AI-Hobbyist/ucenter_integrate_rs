//! 客户端配置。

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct UcenterConfig {
    /// UCenter 根地址，或完整的 `.../api/index.php` 地址。
    pub api_url: String,
    pub app_id: String,
    pub secret: String,
    pub timeout: Duration,
}

impl UcenterConfig {
    pub fn new(
        api_url: impl Into<String>,
        app_id: impl Into<String>,
        secret: impl Into<String>,
    ) -> Self {
        Self {
            api_url: api_url.into(),
            app_id: app_id.into(),
            secret: secret.into(),
            timeout: Duration::from_secs(10),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// 将配置的地址规范为 RESTful 入口 `.../api/index.php`。
pub fn rest_endpoint(api_url: &str) -> String {
    let trimmed = api_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/api/index.php") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/index.php")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_endpoint_normalizes_trailing_slash() {
        assert_eq!(
            rest_endpoint("https://uc.example.com/"),
            "https://uc.example.com/api/index.php"
        );
        assert_eq!(
            rest_endpoint("https://uc.example.com/ucenter/api/index.php"),
            "https://uc.example.com/ucenter/api/index.php"
        );
    }
}
