use chrono::NaiveDate;
use serde::Serialize;

use crate::db::{BudgetRepository, CategoryRepository, Database, Repository};
use crate::error::{AppError, AppResult};
use crate::models::{Budget, BudgetPeriod, CategoryKind};

#[derive(Debug, Serialize)]
pub struct BudgetStatus {
    pub budget_id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub category_icon: String,
    pub amount_cents: i64,
    pub spent_cents: i64,
    pub remaining_cents: i64,
    pub progress_pct: f64,
}

pub struct BudgetService<'a> {
    db: &'a Database,
}

impl<'a> BudgetService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list(&self) -> AppResult<Vec<Budget>> {
        BudgetRepository::new(self.db.conn()).find_all()
    }

    /// 设置预算，可选 from_date 用于时间范围隔离
    pub fn set(&self, category_id: i64, amount_yuan: f64, period_str: &str, from_date: Option<NaiveDate>) -> AppResult<i64> {
        let cat = CategoryRepository::new(self.db.conn())
            .find_by_id(category_id)?
            .ok_or_else(|| AppError::NotFound(format!("分类 ID={category_id} 不存在")))?;

        if cat.kind != CategoryKind::Expense {
            return Err(AppError::Validation("只能为支出分类设置预算".into()));
        }

        let amount_cents = (amount_yuan * 100.0).round() as i64;
        if amount_cents <= 0 {
            return Err(AppError::Validation("预算金额必须大于0".into()));
        }

        let period = match period_str {
            "monthly" | "月" => BudgetPeriod::Monthly,
            "weekly" | "周" => BudgetPeriod::Weekly,
            _ => return Err(AppError::Validation("无效周期，请使用monthly或weekly".into())),
        };

        let budget_repo = BudgetRepository::new(self.db.conn());
        // 查找是否已有同分类+同时间段的预算
        if let Some(mut existing) = budget_repo.find_by_category(category_id)? {
            existing.amount_cents = amount_cents;
            existing.period = period;
            existing.start_date = from_date.unwrap_or(existing.start_date);
            budget_repo.update(&existing)?;
            Ok(existing.id.unwrap())
        } else {
            let budget = Budget {
                id: None,
                category_id,
                amount_cents,
                period,
                start_date: from_date.unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
                end_date: None,
                created_at: None,
            };
            budget_repo.create(&budget)
        }
    }

    /// 预算执行状态，按时间范围筛选
    pub fn status(&self, from_date: Option<NaiveDate>) -> AppResult<Vec<BudgetStatus>> {
        let budgets = BudgetRepository::new(self.db.conn()).find_all()?;
        let mut result = Vec::new();

        for budget in budgets {
            let cat = match CategoryRepository::new(self.db.conn()).find_by_id(budget.category_id)? {
                Some(c) => c,
                None => continue,
            };

            // 计算该分类在指定时间范围的支出
            let spent: i64 = if let Some(from) = from_date {
                self.db.conn().query_row(
                    "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE category_id=?1 AND type='expense' AND txn_date>=?2",
                    rusqlite::params![budget.category_id, from.to_string()],
                    |row| row.get(0),
                )?
            } else {
                self.db.conn().query_row(
                    "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE category_id=?1 AND type='expense'",
                    rusqlite::params![budget.category_id],
                    |row| row.get(0),
                )?
            };

            let remaining = (budget.amount_cents - spent).max(0);
            let progress_pct = if budget.amount_cents > 0 {
                (spent as f64 / budget.amount_cents as f64 * 100.0).min(100.0)
            } else { 0.0 };

            result.push(BudgetStatus {
                budget_id: budget.id.unwrap(),
                category_id: budget.category_id,
                category_name: cat.name,
                category_icon: cat.icon,
                amount_cents: budget.amount_cents,
                spent_cents: spent,
                remaining_cents: remaining,
                progress_pct: (progress_pct * 10.0).round() / 10.0,
            });
        }
        Ok(result)
    }

    pub fn alerts(&self, threshold_pct: f64) -> AppResult<Vec<BudgetStatus>> {
        let all = self.status(None)?;
        Ok(all.into_iter().filter(|s| s.progress_pct >= threshold_pct).collect())
    }

    /// 删除预算
    pub fn delete(&self, budget_id: i64) -> AppResult<()> {
        if !BudgetRepository::new(self.db.conn()).delete(budget_id)? {
            return Err(AppError::NotFound(format!("预算 ID={budget_id} 不存在")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::engine::TxnService;
    use crate::models::Account;
    use crate::db::{AccountRepository, Repository};

    #[test]
    fn test_budget_tracking() {
        let db = Database::open_memory().unwrap();
        let acc_id = AccountRepository::new(db.conn())
            .create(&Account::new("工资卡", crate::models::AccountKind::BankCard, "CNY", 1_000_000))
            .unwrap();
        let budget_svc = BudgetService::new(&db);
        budget_svc.set(5, 1000.0, "monthly", None).unwrap();
        let txn_svc = TxnService::new(&db);
        txn_svc.create(acc_id, 5, 350.0, "expense", "聚餐", None).unwrap();
        let status = budget_svc.status(None).unwrap();
        let food = status.iter().find(|s| s.category_id == 5).unwrap();
        assert_eq!(food.spent_cents, 35_000);
        assert_eq!(food.progress_pct, 35.0);
    }
}
