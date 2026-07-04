use super::{BillImportAdapter, CsvParser, ImportResult, ImportTransaction};
use crate::error::AppResult;

/// 微信账单适配器 — 处理真实微信导出格式
pub struct WechatAdapter;

impl BillImportAdapter for WechatAdapter {
    fn format_name(&self) -> &'static str {
        "微信"
    }

    fn detect(&self, header: &str) -> bool {
        header.contains("微信支付账单明细")
    }

    fn parse(&self, content: &str) -> AppResult<ImportResult> {
        // 跳过元数据行，找到真正的表头和数据开始位置
        let lines: Vec<&str> = content.lines().collect();
        let header_idx = lines
            .iter()
            .position(|l| l.starts_with("交易时间,交易类型,交易对方"))
            .unwrap_or(0);

        if header_idx == 0 || header_idx + 1 >= lines.len() {
            return Ok(ImportResult {
                format_name: self.format_name().to_string(),
                transactions: vec![],
                warnings: vec!["未找到有效的交易数据".into()],
            });
        }

        // 重组CSV：表头 + 数据行
        let csv_content: String = std::iter::once(lines[header_idx])
            .chain(lines[header_idx + 1..].iter().copied())
            .collect::<Vec<_>>()
            .join("\n");

        let (headers, rows) = CsvParser::parse(&csv_content)?;

        let date_col = CsvParser::find_column(&headers, &["交易时间"]);
        let inout_col = CsvParser::find_column(&headers, &["收/支"]);
        let desc_col = CsvParser::find_column(&headers, &["商品"]);
        let amount_col = CsvParser::find_column(&headers, &["金额"]);
        let counterparty_col = CsvParser::find_column(&headers, &["交易对方"]);

        let mut transactions = Vec::new();
        let warnings = Vec::new();

        for row in &rows {
            let date_str = CsvParser::get_column(row, date_col).unwrap_or("").trim().to_string();
            let inout = CsvParser::get_column(row, inout_col).unwrap_or("").trim().to_string();
            let desc = CsvParser::get_column(row, desc_col).unwrap_or("").trim().to_string();
            let amount_raw = CsvParser::get_column(row, amount_col).unwrap_or("0").trim().to_string();
            let counterparty = CsvParser::get_column(row, counterparty_col)
                .map(|s| s.trim().to_string());

            // 解析日期: "2026-06-30 19:05:19" -> 取前10位
            let date = chrono::NaiveDate::parse_from_str(
                &date_str.chars().take(10).collect::<String>(),
                "%Y-%m-%d",
            )
            .unwrap_or_else(|_| chrono::Local::now().date_naive());

            // 解析金额: "¥311.00" 或 "\"¥3,000.00\""（含引号和千位逗号）
            let cleaned = amount_raw
                .replace('"', "")
                .replace('¥', "")
                .replace(',', "")
                .trim()
                .to_string();
            let amount_yuan: f64 = cleaned.parse().unwrap_or(0.0);
            let amount_cents = (amount_yuan.abs() * 100.0).round() as i64;

            // 判断收支
            let txn_type = if inout.contains("收入") { "income" } else { "expense" };

            // 描述清理
            let description = if desc.is_empty() {
                "微信支付".to_string()
            } else if desc.chars().count() > 30 {
                desc.chars().take(30).collect::<String>()
            } else {
                desc
            };

            transactions.push(ImportTransaction {
                txn_date: date,
                amount_cents,
                txn_type: txn_type.to_string(),
                description,
                counterparty,
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
