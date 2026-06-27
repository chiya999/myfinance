use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;

use crate::db::{AccountRepository, Database};
use crate::error::AppResult;
use crate::models::CategoryKind;

#[derive(Debug, Serialize)]
pub struct SummaryReport {
    pub total_income: i64,
    pub total_expense: i64,
    pub net: i64,
    pub transaction_count: usize,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct CategoryBreakdown {
    pub category_name: String,
    pub category_icon: String,
    pub amount_cents: i64,
    pub percentage: f64,
    pub transaction_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TrendPoint {
    pub date: NaiveDate,
    pub income_cents: i64,
    pub expense_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct AccountSnapshot {
    pub account_name: String,
    pub account_kind: String,
    pub balance_cents: i64,
    pub month_income: i64,
    pub month_expense: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum TrendGranularity {
    Daily,
    Weekly,
    Monthly,
}

/// 报表生成服务
pub struct ReportService<'a> {
    db: &'a Database,
}

impl<'a> ReportService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 收支总览
    pub fn summary(&self, from: NaiveDate, to: NaiveDate) -> AppResult<SummaryReport> {
        let conn = self.db.conn();

        let total_income: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE type = 'income' AND txn_date >= ?1 AND txn_date <= ?2",
            rusqlite::params![from.to_string(), to.to_string()],
            |row| row.get(0),
        )?;

        let total_expense: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE type = 'expense' AND txn_date >= ?1 AND txn_date <= ?2",
            rusqlite::params![from.to_string(), to.to_string()],
            |row| row.get(0),
        )?;

        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM transactions WHERE txn_date >= ?1 AND txn_date <= ?2",
            rusqlite::params![from.to_string(), to.to_string()],
            |row| row.get(0),
        )?;

        Ok(SummaryReport {
            total_income,
            total_expense,
            net: total_income - total_expense,
            transaction_count: count,
            period_start: from,
            period_end: to,
        })
    }

    /// 按分类统计
    pub fn category_breakdown(
        &self,
        kind: CategoryKind,
        from: NaiveDate,
        to: NaiveDate,
    ) -> AppResult<Vec<CategoryBreakdown>> {
        let conn = self.db.conn();
        let kind_str = kind.to_db();

        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(t.amount_cents), 0) FROM transactions t WHERE t.type = ?1 AND t.txn_date >= ?2 AND t.txn_date <= ?3",
            rusqlite::params![kind_str, from.to_string(), to.to_string()],
            |row| row.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT c.name, c.icon, COALESCE(SUM(t.amount_cents), 0) AS amt, COUNT(*) AS cnt
             FROM transactions t
             JOIN categories c ON c.id = t.category_id
             WHERE t.type = ?1 AND t.txn_date >= ?2 AND t.txn_date <= ?3
             GROUP BY t.category_id
             ORDER BY amt DESC",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![kind_str, from.to_string(), to.to_string()],
            |row| {
                Ok(CategoryBreakdown {
                    category_name: row.get(0)?,
                    category_icon: row.get::<_, String>(1).unwrap_or_default(),
                    amount_cents: row.get(2)?,
                    percentage: if total > 0 {
                        row.get::<_, i64>(2)? as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    },
                    transaction_count: row.get(3)?,
                })
            },
        )?;

        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(crate::error::AppError::from)
    }

    /// 收支趋势
    pub fn trend(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        granularity: TrendGranularity,
    ) -> AppResult<Vec<TrendPoint>> {
        let conn = self.db.conn();
        let date_format = match granularity {
            TrendGranularity::Daily => "%Y-%m-%d",
            TrendGranularity::Weekly => "%Y-%W",
            TrendGranularity::Monthly => "%Y-%m",
        };

        let sql = format!(
            "SELECT txn_date,
                    COALESCE(SUM(CASE WHEN type='income' THEN amount_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='expense' THEN amount_cents ELSE 0 END), 0)
             FROM transactions
             WHERE txn_date >= ?1 AND txn_date <= ?2
             GROUP BY strftime('{}', txn_date)
             ORDER BY txn_date ASC",
            date_format
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![from.to_string(), to.to_string()], |row| {
            let date_str: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap_or_else(|_| from);
            Ok(TrendPoint {
                date,
                income_cents: row.get(1)?,
                expense_cents: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(crate::error::AppError::from)
    }

    /// 账户快照
    pub fn account_snapshot(&self) -> AppResult<Vec<AccountSnapshot>> {
        let acc_repo = AccountRepository::new(self.db.conn());
        let accounts = acc_repo.find_active()?;
        let today = Local::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

        use chrono::Local;

        let mut snapshots = Vec::new();
        for acc in accounts {
            let conn = self.db.conn();
            let month_income: i64 = conn.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE account_id = ?1 AND type = 'income' AND txn_date >= ?2",
                rusqlite::params![acc.id, month_start.to_string()],
                |row| row.get(0),
            )?;
            let month_expense: i64 = conn.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE account_id = ?1 AND type = 'expense' AND txn_date >= ?2",
                rusqlite::params![acc.id, month_start.to_string()],
                |row| row.get(0),
            )?;

            snapshots.push(AccountSnapshot {
                account_name: acc.name,
                account_kind: acc.kind.display_name().to_string(),
                balance_cents: acc.balance_cents,
                month_income,
                month_expense,
            });
        }
        Ok(snapshots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::db::{AccountRepository, Repository};
    use crate::engine::TxnService;
    use crate::models::Account;

    #[test]
    fn test_summary_report() {
        let db = Database::open_memory().unwrap();
        let acc = Account::new(
            "工资卡",
            crate::models::AccountKind::BankCard,
            "CNY",
            1_000_000,
        );
        let acc_id = AccountRepository::new(db.conn()).create(&acc).unwrap();

        let txn_svc = TxnService::new(&db);
        txn_svc
            .create(acc_id, 1, 15000.0, "income", "工资", Some("2025-06-01"))
            .unwrap();
        txn_svc
            .create(acc_id, 5, 350.0, "expense", "午餐", Some("2025-06-02"))
            .unwrap();
        txn_svc
            .create(acc_id, 9, 88.0, "expense", "电影", Some("2025-06-03"))
            .unwrap();

        let svc = ReportService::new(&db);
        let report = svc
            .summary(
                NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
            )
            .unwrap();

        assert_eq!(report.total_income, 1_500_000);
        assert_eq!(report.total_expense, 43_800);
        assert_eq!(report.transaction_count, 3);
    }
}
