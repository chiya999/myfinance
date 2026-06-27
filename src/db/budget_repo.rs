use chrono::NaiveDate;
use rusqlite::Connection;

use super::Repository;
use crate::error::{AppError, AppResult};
use crate::models::{Budget, BudgetPeriod};

/// 预算仓库
pub struct BudgetRepository<'a> {
    conn: &'a Connection,
}

impl<'a> BudgetRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 查询当前有效的预算（end_date 为 NULL 或 >= 今天）
    pub fn find_active(&self, today: NaiveDate) -> AppResult<Vec<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, amount_cents, period, start_date, end_date, created_at FROM budgets WHERE end_date IS NULL OR end_date >= ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([today.to_string()], |row| {
            let period_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                amount_cents: row.get(2)?,
                period: BudgetPeriod::from_db(&period_str).unwrap_or(BudgetPeriod::Monthly),
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(4)?, "%Y-%m-%d")
                    .unwrap_or(today),
                end_date: row
                    .get::<_, Option<String>>(5)?
                    .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap_or(today)),
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }

    /// 查询某分类在指定周期的预算
    pub fn find_by_category(&self, category_id: i64) -> AppResult<Option<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, amount_cents, period, start_date, end_date, created_at FROM budgets WHERE category_id = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map([category_id], |row| {
            let period_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                amount_cents: row.get(2)?,
                period: BudgetPeriod::from_db(&period_str).unwrap_or(BudgetPeriod::Monthly),
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(4)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                end_date: row.get::<_, Option<String>>(5)?.map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                }),
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

impl<'a> Repository<Budget> for BudgetRepository<'a> {
    fn create(&self, budget: &Budget) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO budgets (category_id, amount_cents, period, start_date, end_date) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                budget.category_id,
                budget.amount_cents,
                budget.period.to_db(),
                budget.start_date.to_string(),
                budget.end_date.map(|d| d.to_string()),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_by_id(&self, id: i64) -> AppResult<Option<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, amount_cents, period, start_date, end_date, created_at FROM budgets WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let period_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                amount_cents: row.get(2)?,
                period: BudgetPeriod::from_db(&period_str).unwrap_or(BudgetPeriod::Monthly),
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(4)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                end_date: row.get::<_, Option<String>>(5)?.map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                }),
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> AppResult<Vec<Budget>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category_id, amount_cents, period, start_date, end_date, created_at FROM budgets ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let period_str: String = row.get(3)?;
            Ok(Budget {
                id: Some(row.get(0)?),
                category_id: row.get(1)?,
                amount_cents: row.get(2)?,
                period: BudgetPeriod::from_db(&period_str).unwrap_or(BudgetPeriod::Monthly),
                start_date: NaiveDate::parse_from_str(&row.get::<_, String>(4)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                end_date: row.get::<_, Option<String>>(5)?.map(|s| {
                    NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                }),
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }

    fn update(&self, budget: &Budget) -> AppResult<bool> {
        let affected = self.conn.execute(
            "UPDATE budgets SET category_id = ?1, amount_cents = ?2, period = ?3, start_date = ?4, end_date = ?5 WHERE id = ?6",
            rusqlite::params![
                budget.category_id,
                budget.amount_cents,
                budget.period.to_db(),
                budget.start_date.to_string(),
                budget.end_date.map(|d| d.to_string()),
                budget.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: i64) -> AppResult<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM budgets WHERE id = ?1", [id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_budget_crud() {
        let db = Database::open_memory().unwrap();
        let repo = BudgetRepository::new(db.conn());

        let budget = Budget {
            id: None,
            category_id: 5,        // 餐饮
            amount_cents: 100_000, // ¥1000
            period: BudgetPeriod::Monthly,
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: None,
            created_at: None,
        };

        let id = repo.create(&budget).unwrap();
        assert!(id > 0);

        let found = repo.find_by_id(id).unwrap().unwrap();
        assert_eq!(found.amount_cents, 100_000);
        assert_eq!(found.period, BudgetPeriod::Monthly);
    }
}
