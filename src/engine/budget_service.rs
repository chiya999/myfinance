use chrono::{Datelike, Local, NaiveDate};
use serde::Serialize;

use crate::db::{BudgetRepository, CategoryRepository, Database, Repository};
use crate::error::{AppError, AppResult};
use crate::models::{Budget, BudgetPeriod, CategoryKind};

/// 预算执行状态
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

/// 预算管理服务
pub struct BudgetService<'a> {
    db: &'a Database,
}

impl<'a> BudgetService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 列出所有预算
    pub fn list(&self) -> AppResult<Vec<Budget>> {
        BudgetRepository::new(self.db.conn()).find_all()
    }

    /// 设置/更新预算
    pub fn set(&self, category_id: i64, amount_yuan: f64, period_str: &str) -> AppResult<i64> {
        // 校验分类存在且为支出类别
        let cat = CategoryRepository::new(self.db.conn())
            .find_by_id(category_id)?
            .ok_or_else(|| AppError::NotFound(format!("分类 ID={category_id} 不存在")))?;

        if cat.kind != CategoryKind::Expense {
            return Err(AppError::Validation("只能为支出分类设置预算".into()));
        }

        let period = match period_str {
            "monthly" | "月" => BudgetPeriod::Monthly,
            "weekly" | "周" => BudgetPeriod::Weekly,
            _ => {
                return Err(AppError::Validation(format!(
                    "无效周期: {period_str}, 请使用 monthly 或 weekly"
                )));
            }
        };

        let amount_cents = (amount_yuan * 100.0).round() as i64;
        if amount_cents <= 0 {
            return Err(AppError::Validation("预算金额必须大于 0".into()));
        }

        let today = Local::now().date_naive();
        let start_date = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

        // 检查是否已存在，存在则更新
        let budget_repo = BudgetRepository::new(self.db.conn());
        if let Some(existing) = budget_repo.find_by_category(category_id)? {
            let mut updated = existing;
            updated.amount_cents = amount_cents;
            updated.period = period;
            budget_repo.update(&updated)?;
            Ok(updated.id.unwrap())
        } else {
            let budget = Budget {
                id: None,
                category_id,
                amount_cents,
                period,
                start_date,
                end_date: None,
                created_at: None,
            };
            budget_repo.create(&budget)
        }
    }

    /// 预算执行状态
    pub fn status(&self) -> AppResult<Vec<BudgetStatus>> {
        let budgets = BudgetRepository::new(self.db.conn()).find_all()?;
        let today = Local::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

        let mut result = Vec::new();
        for budget in budgets {
            let cat = CategoryRepository::new(self.db.conn())
                .find_by_id(budget.category_id)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("分类 ID={} 不存在", budget.category_id))
                })?;

            // 计算当月该分类的支出总额
            let spent: i64 = self.db.conn().query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE category_id = ?1 AND type = 'expense' AND txn_date >= ?2",
                rusqlite::params![budget.category_id, month_start.to_string()],
                |row| row.get(0),
            )?;

            let remaining = (budget.amount_cents - spent).max(0);
            let progress_pct = if budget.amount_cents > 0 {
                (spent as f64 / budget.amount_cents as f64 * 100.0).min(100.0)
            } else {
                0.0
            };

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

    /// 超支告警
    pub fn alerts(&self, threshold_pct: f64) -> AppResult<Vec<BudgetStatus>> {
        let all = self.status()?;
        Ok(all
            .into_iter()
            .filter(|s| s.progress_pct >= threshold_pct)
            .collect())
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
    fn test_budget_tracking() {
        let db = Database::open_memory().unwrap();
        let acc_id = AccountRepository::new(db.conn())
            .create(&Account::new(
                "工资卡",
                crate::models::AccountKind::BankCard,
                "CNY",
                1_000_000,
            ))
            .unwrap();

        // 设置餐饮预算 ¥1000
        let budget_svc = BudgetService::new(&db);
        budget_svc.set(5, 1000.0, "monthly").unwrap(); // category 5 = 餐饮

        // 记一笔餐饮支出 ¥350
        let txn_svc = TxnService::new(&db);
        txn_svc
            .create(acc_id, 5, 350.0, "expense", "聚餐", None)
            .unwrap();

        // 检查状态
        let status = budget_svc.status().unwrap();
        let food_status = status.iter().find(|s| s.category_id == 5).unwrap();
        assert_eq!(food_status.spent_cents, 35_000);
        assert_eq!(food_status.progress_pct, 35.0);

        // 超支告警 (阈值 30%)
        let alerts = budget_svc.alerts(30.0).unwrap();
        assert!(alerts.iter().any(|s| s.category_id == 5));
    }
}
