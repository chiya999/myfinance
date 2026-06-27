use chrono::Datelike;

mod account;
mod budget;
mod category;
mod transaction;

pub use account::{Account, AccountKind};
pub use budget::{Budget, BudgetPeriod};
pub use category::{Category, CategoryKind};
pub use transaction::{Transaction, TxnType};

/// 金额显示辅助: 分 -> 元字符串
pub fn cents_to_yuan(cents: i64) -> String {
    format!("{:.2}", cents as f64 / 100.0)
}

/// 字符串解析为金额(分): "100.50" -> 10050
pub fn parse_cents(s: &str) -> Result<i64, crate::error::AppError> {
    let yuan: f64 = s
        .parse()
        .map_err(|_| crate::error::AppError::Validation(format!("无效金额: {s}")))?;
    Ok((yuan * 100.0).round() as i64)
}

/// 从 NaiveDate 获取当月第一天
pub fn first_of_month(date: chrono::NaiveDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

/// 从 NaiveDate 获取当月最后一天
pub fn last_of_month(date: chrono::NaiveDate) -> chrono::NaiveDate {
    first_of_month(date)
        .checked_add_months(chrono::Months::new(1))
        .unwrap_or(date)
        .pred_opt()
        .unwrap_or(date)
}
