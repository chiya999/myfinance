use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 默认数据库文件路径
    pub database_path: Option<String>,
    /// 默认货币
    #[serde(default = "default_currency")]
    pub default_currency: String,
    /// 报表默认天数
    #[serde(default = "default_report_days")]
    pub report_period_days: i64,
    /// 预算告警阈值
    #[serde(default = "default_alert_threshold")]
    pub budget_alert_threshold: f64,
}

fn default_currency() -> String {
    "CNY".into()
}
fn default_report_days() -> i64 {
    30
}
fn default_alert_threshold() -> f64 {
    80.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            database_path: None,
            default_currency: "CNY".into(),
            report_period_days: 30,
            budget_alert_threshold: 80.0,
        }
    }
}
