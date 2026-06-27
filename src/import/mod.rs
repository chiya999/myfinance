mod alipay_adapter;
mod csv_parser;
mod wechat_adapter;

use std::path::Path;

pub use alipay_adapter::AlipayAdapter;
pub use csv_parser::CsvParser;
pub use wechat_adapter::WechatAdapter;

use crate::error::AppResult;

/// 从外部账单解析到的交易记录
#[derive(Debug, Clone)]
pub struct ImportTransaction {
    pub txn_date: chrono::NaiveDate,
    pub amount_cents: i64,
    pub txn_type: String, // "income" | "expense"
    pub description: String,
    pub counterparty: Option<String>,
    pub raw_category_hint: Option<String>,
}

/// 导入结果
#[derive(Debug)]
pub struct ImportResult {
    pub format_name: String,
    pub transactions: Vec<ImportTransaction>,
    pub warnings: Vec<String>,
}

/// 账单导入适配器 trait
pub trait BillImportAdapter {
    fn format_name(&self) -> &'static str;
    fn detect(&self, header: &str) -> bool;
    fn parse(&self, content: &str) -> AppResult<ImportResult>;
}

/// 自动检测文件格式并导入
pub fn import_file(path: &Path) -> AppResult<ImportResult> {
    let content = std::fs::read_to_string(path)?;
    let first_line = content.lines().next().unwrap_or("");

    let adapters: Vec<Box<dyn BillImportAdapter>> =
        vec![Box::new(AlipayAdapter), Box::new(WechatAdapter)];

    for adapter in &adapters {
        if adapter.detect(first_line) {
            return adapter.parse(&content);
        }
    }

    Err(crate::error::AppError::Validation(format!(
        "无法识别文件格式: {}。支持: 支付宝、微信账单",
        path.display()
    )))
}
