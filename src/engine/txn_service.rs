use chrono::NaiveDate;
use std::str::FromStr;

use crate::db::{
    AccountRepository, CategoryRepository, Database, Repository, TransactionRepository,
};
use crate::error::{AppError, AppResult};
use crate::models::{Transaction, TxnType};

/// 交易管理服务 —— 所有写操作保证事务性
pub struct TxnService<'a> {
    db: &'a Database,
    txn_repo: TransactionRepository<'a>,
}

impl<'a> TxnService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            txn_repo: TransactionRepository::new(db.conn()),
            db,
        }
    }

    /// 添加收入/支出 —— 事务性：INSERT txn + UPDATE account balance
    pub fn create(
        &self,
        account_id: i64,
        category_id: i64,
        amount_yuan: f64,
        txn_type_str: &str,
        description: &str,
        date_str: Option<&str>,
    ) -> AppResult<i64> {
        let txn_type: TxnType = txn_type_str
            .parse()
            .map_err(|e: String| AppError::Validation(e))?;

        if !matches!(txn_type, TxnType::Income | TxnType::Expense) {
            return Err(AppError::Validation(
                "请使用 income 或 expense 类型，转账请用 transfer 命令".into(),
            ));
        }

        let amount_cents = (amount_yuan * 100.0).round() as i64;
        if amount_cents <= 0 {
            return Err(AppError::Validation("金额必须大于 0".into()));
        }

        // 校验账户存在
        let acc_repo = AccountRepository::new(self.db.conn());
        let mut account = acc_repo
            .find_by_id(account_id)?
            .ok_or_else(|| AppError::NotFound(format!("账户 ID={account_id} 不存在")))?;

        if !account.is_active {
            return Err(AppError::Validation(format!(
                "账户 '{}' 已停用",
                account.name
            )));
        }

        // 校验分类存在
        let cat_repo = CategoryRepository::new(self.db.conn());
        cat_repo
            .find_by_id(category_id)?
            .ok_or_else(|| AppError::NotFound(format!("分类 ID={category_id} 不存在")))?;

        let date = date_str
            .map(|s| {
                NaiveDate::from_str(s).map_err(|_| AppError::Validation(format!("无效日期: {s}")))
            })
            .transpose()?
            .unwrap_or_else(|| chrono::Local::now().date_naive());

        let txn = Transaction::new_income_expense(
            account_id,
            category_id,
            txn_type,
            amount_cents,
            description,
            date,
        );

        // 更新余额
        match txn_type {
            TxnType::Income => account.balance_cents += amount_cents,
            TxnType::Expense => account.balance_cents -= amount_cents,
            _ => unreachable!(),
        }

        // 事务性写入
        let conn = self.db.conn();
        conn.execute("BEGIN IMMEDIATE", [])?;

        let txn_id = match self.txn_repo.create(&txn) {
            Ok(id) => id,
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(e);
            }
        };

        if let Err(e) = acc_repo.update_balance(account_id, account.balance_cents) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }

        conn.execute("COMMIT", [])?;
        Ok(txn_id)
    }

    /// 转账 —— 事务性：INSERT transfer + UPDATE from_balance + UPDATE to_balance
    pub fn transfer(
        &self,
        from_account_id: i64,
        to_account_id: i64,
        amount_yuan: f64,
        description: &str,
        date_str: Option<&str>,
    ) -> AppResult<i64> {
        if from_account_id == to_account_id {
            return Err(AppError::Validation("不能向同一账户转账".into()));
        }

        let amount_cents = (amount_yuan * 100.0).round() as i64;
        if amount_cents <= 0 {
            return Err(AppError::Validation("金额必须大于 0".into()));
        }

        let acc_repo = AccountRepository::new(self.db.conn());

        let mut from_acc = acc_repo
            .find_by_id(from_account_id)?
            .ok_or_else(|| AppError::NotFound(format!("转出账户 ID={from_account_id} 不存在")))?;
        let to_acc = acc_repo
            .find_by_id(to_account_id)?
            .ok_or_else(|| AppError::NotFound(format!("转入账户 ID={to_account_id} 不存在")))?;

        if !from_acc.is_active || !to_acc.is_active {
            return Err(AppError::Validation(
                "转账双方账户都必须处于活跃状态".into(),
            ));
        }

        if from_acc.balance_cents < amount_cents
            && from_acc.kind != crate::models::AccountKind::CreditCard
        {
            return Err(AppError::Validation(format!(
                "余额不足: 账户 '{}' 余额 ¥{}，需要 ¥{}",
                from_acc.name,
                crate::models::cents_to_yuan(from_acc.balance_cents),
                crate::models::cents_to_yuan(amount_cents),
            )));
        }

        let date = date_str
            .map(|s| {
                NaiveDate::from_str(s).map_err(|_| AppError::Validation(format!("无效日期: {s}")))
            })
            .transpose()?
            .unwrap_or_else(|| chrono::Local::now().date_naive());

        let txn = Transaction::new_transfer(
            from_account_id,
            to_account_id,
            amount_cents,
            description,
            date,
        );

        from_acc.balance_cents -= amount_cents;
        let new_to_balance = to_acc.balance_cents + amount_cents;

        let conn = self.db.conn();
        conn.execute("BEGIN IMMEDIATE", [])?;

        let txn_id = match self.txn_repo.create(&txn) {
            Ok(id) => id,
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                return Err(e);
            }
        };

        if let Err(e) = acc_repo.update_balance(from_account_id, from_acc.balance_cents) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }
        if let Err(e) = acc_repo.update_balance(to_account_id, new_to_balance) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }

        conn.execute("COMMIT", [])?;
        Ok(txn_id)
    }

    /// 列出交易
    pub fn list(
        &self,
        account_id: Option<i64>,
        category_id: Option<i64>,
        txn_type: Option<TxnType>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        min_amount: Option<i64>,
        max_amount: Option<i64>,
        limit: usize,
    ) -> AppResult<Vec<Transaction>> {
        self.txn_repo.find_filtered(
            account_id,
            category_id,
            txn_type,
            from_date,
            to_date,
            min_amount,
            max_amount,
            limit,
        )
    }

    /// 删除交易并冲正余额
    pub fn delete(&self, txn_id: i64) -> AppResult<()> {
        let txn = self
            .txn_repo
            .find_by_id(txn_id)?
            .ok_or_else(|| AppError::NotFound(format!("交易 ID={txn_id} 不存在")))?;

        let acc_repo = AccountRepository::new(self.db.conn());
        let conn = self.db.conn();
        conn.execute("BEGIN IMMEDIATE", [])?;

        // 冲正余额
        match txn.txn_type {
            TxnType::Income => {
                let mut acc = acc_repo.find_by_id(txn.account_id)?.unwrap();
                acc.balance_cents -= txn.amount_cents;
                acc_repo.update_balance(acc.id.unwrap(), acc.balance_cents)?;
            }
            TxnType::Expense => {
                let mut acc = acc_repo.find_by_id(txn.account_id)?.unwrap();
                acc.balance_cents += txn.amount_cents;
                acc_repo.update_balance(acc.id.unwrap(), acc.balance_cents)?;
            }
            TxnType::Transfer => {
                let mut from_acc = acc_repo.find_by_id(txn.account_id)?.unwrap();
                from_acc.balance_cents += txn.amount_cents;
                acc_repo.update_balance(from_acc.id.unwrap(), from_acc.balance_cents)?;

                if let Some(to_id) = txn.to_account_id {
                    let mut to_acc = acc_repo.find_by_id(to_id)?.unwrap();
                    to_acc.balance_cents -= txn.amount_cents;
                    acc_repo.update_balance(to_acc.id.unwrap(), to_acc.balance_cents)?;
                }
            }
        }

        if let Err(e) = self.txn_repo.delete(txn_id) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }

        conn.execute("COMMIT", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{Account, AccountKind};

    fn setup() -> (Database, i64, i64) {
        let db = Database::open_memory().unwrap();
        let acc_repo = AccountRepository::new(db.conn());
        let id1 = acc_repo
            .create(&Account::new(
                "工资卡",
                AccountKind::BankCard,
                "CNY",
                500_000,
            ))
            .unwrap();
        let id2 = acc_repo
            .create(&Account::new("现金", AccountKind::Wallet, "CNY", 50_000))
            .unwrap();
        (db, id1, id2)
    }

    #[test]
    fn test_income_increases_balance() {
        let (db, acc_id, _) = setup();
        let svc = TxnService::new(&db);

        svc.create(acc_id, 1, 1500.0, "income", "工资", None)
            .unwrap(); // category 1 = 工资
        let acc = AccountRepository::new(db.conn())
            .find_by_id(acc_id)
            .unwrap()
            .unwrap();
        assert_eq!(acc.balance_cents, 650_000); // 500000 + 150000
    }

    #[test]
    fn test_expense_decreases_balance() {
        let (db, acc_id, _) = setup();
        let svc = TxnService::new(&db);

        svc.create(acc_id, 5, 45.5, "expense", "午餐", None)
            .unwrap(); // category 5 = 餐饮
        let acc = AccountRepository::new(db.conn())
            .find_by_id(acc_id)
            .unwrap()
            .unwrap();
        assert_eq!(acc.balance_cents, 495_450); // 500000 - 4550
    }

    #[test]
    fn test_transfer_moves_money() {
        let (db, from_id, to_id) = setup();
        let svc = TxnService::new(&db);

        svc.transfer(from_id, to_id, 200.0, "取现", None).unwrap();
        let from_acc = AccountRepository::new(db.conn())
            .find_by_id(from_id)
            .unwrap()
            .unwrap();
        let to_acc = AccountRepository::new(db.conn())
            .find_by_id(to_id)
            .unwrap()
            .unwrap();
        assert_eq!(from_acc.balance_cents, 480_000); // 500000 - 20000
        assert_eq!(to_acc.balance_cents, 70_000); // 50000 + 20000
    }

    #[test]
    fn test_delete_reverses_balance() {
        let (db, acc_id, _) = setup();
        let svc = TxnService::new(&db);

        let txn_id = svc
            .create(acc_id, 5, 100.0, "expense", "测试支出", None)
            .unwrap();
        let before_delete = AccountRepository::new(db.conn())
            .find_by_id(acc_id)
            .unwrap()
            .unwrap()
            .balance_cents;

        svc.delete(txn_id).unwrap();
        let after_delete = AccountRepository::new(db.conn())
            .find_by_id(acc_id)
            .unwrap()
            .unwrap()
            .balance_cents;
        assert_eq!(after_delete, before_delete + 10_000); // 恢复了100元
    }
}
