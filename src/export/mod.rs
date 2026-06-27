mod csv_exporter;
mod json_exporter;

use std::io::Write;

pub use csv_exporter::CsvExporter;
pub use json_exporter::JsonExporter;

use crate::error::AppResult;
use crate::models::{Account, Budget, Category, Transaction};
use chrono::Local;

/// 导出数据集合
#[derive(Debug, serde::Serialize)]
pub struct ExportData {
    pub exported_at: String,
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
    pub categories: Vec<Category>,
    pub budgets: Vec<Budget>,
}

impl ExportData {
    pub fn new(
        accounts: Vec<Account>,
        transactions: Vec<Transaction>,
        categories: Vec<Category>,
        budgets: Vec<Budget>,
    ) -> Self {
        Self {
            exported_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            accounts,
            transactions,
            categories,
            budgets,
        }
    }
}

/// 导出器 trait
pub trait Exporter {
    fn export(&self, data: &ExportData, writer: &mut dyn Write) -> AppResult<()>;
    fn format_name(&self) -> &'static str;
}

/// 导出到文件
pub fn export_to_file(path: &std::path::Path, format: &str, data: &ExportData) -> AppResult<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    match format {
        "json" => JsonExporter.export(data, &mut writer),
        "csv" => CsvExporter.export(data, &mut writer),
        _ => Err(crate::error::AppError::Validation(format!(
            "不支持的导出格式: {format}"
        ))),
    }?;

    println!("✅ 导出完成: {}", path.display());
    Ok(())
}
