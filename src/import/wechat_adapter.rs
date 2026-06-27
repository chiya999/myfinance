use super::{BillImportAdapter, CsvParser, ImportResult, ImportTransaction};
use crate::error::AppResult;

/// 微信账单适配器
pub struct WechatAdapter;

impl BillImportAdapter for WechatAdapter {
    fn format_name(&self) -> &'static str {
        "微信"
    }

    fn detect(&self, header: &str) -> bool {
        header.contains("交易时间") && header.contains("交易类型") && header.contains("交易对方")
    }

    fn parse(&self, content: &str) -> AppResult<ImportResult> {
        let (headers, rows) = CsvParser::parse(content)?;

        let date_col = CsvParser::find_column(&headers, &["交易时间"]);
        let type_col = CsvParser::find_column(&headers, &["收/支"]);
        let desc_col = CsvParser::find_column(&headers, &["商品"]);
        let amount_col = CsvParser::find_column(&headers, &["金额"]);
        let counterparty_col = CsvParser::find_column(&headers, &["交易对方"]);

        let mut transactions = Vec::new();
        let mut warnings = Vec::new();

        for row in &rows {
            let date_str = CsvParser::get_column(&row, date_col).unwrap_or("");
            let type_str = CsvParser::get_column(&row, type_col).unwrap_or("");
            let amount_str = CsvParser::get_column(&row, amount_col).unwrap_or("0");
            let desc = CsvParser::get_column(&row, desc_col).unwrap_or("");

            let date =
                chrono::NaiveDate::parse_from_str(&date_str[..date_str.len().min(10)], "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Local::now().date_naive());

            let amount = amount_str.trim().replace('¥', "").replace(',', "");
            let amount_yuan: f64 = amount.parse().unwrap_or(0.0);
            let amount_cents = (amount_yuan.abs() * 100.0).round() as i64;

            let txn_type = if type_str.contains("收入") {
                "income"
            } else {
                "expense"
            };

            transactions.push(ImportTransaction {
                txn_date: date,
                amount_cents,
                txn_type: txn_type.to_string(),
                description: if desc.is_empty() {
                    "微信支付".to_string()
                } else {
                    desc.to_string()
                },
                counterparty: CsvParser::get_column(&row, counterparty_col).map(|s| s.to_string()),
                raw_category_hint: None,
            });
        }

        Ok(ImportResult {
            format_name: self.format_name().to_string(),
            transactions,
            warnings,
        })
    }
}
