use crate::db::{AccountRepository, Database, Repository};
use crate::error::{AppError, AppResult};
use crate::models::{Account, AccountKind};

/// 账户管理服务
pub struct AccountService<'a> {
    repo: AccountRepository<'a>,
}

impl<'a> AccountService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            repo: AccountRepository::new(db.conn()),
        }
    }

    /// 列出账户
    pub fn list(&self, include_inactive: bool) -> AppResult<Vec<Account>> {
        if include_inactive {
            self.repo.find_all()
        } else {
            self.repo.find_active()
        }
    }

    /// 创建账户
    pub fn create(
        &self,
        name: &str,
        kind_str: &str,
        currency: &str,
        initial_balance_yuan: f64,
    ) -> AppResult<i64> {
        let kind: AccountKind = kind_str
            .parse()
            .map_err(|e: String| AppError::Validation(e))?;
        let initial_balance_cents = (initial_balance_yuan * 100.0).round() as i64;
        let account = Account::new(name, kind, currency, initial_balance_cents);
        self.repo.create(&account)
    }

    /// 获取账户详情
    pub fn get(&self, id: i64) -> AppResult<Account> {
        self.repo
            .find_by_id(id)?
            .ok_or_else(|| AppError::NotFound(format!("账户 ID={id} 不存在")))
    }

    /// 更新账户
    pub fn update(
        &self,
        id: i64,
        name: &str,
        kind_str: &str,
        currency: &str,
        is_active: bool,
    ) -> AppResult<()> {
        let mut account = self.get(id)?;
        let kind: AccountKind = kind_str
            .parse()
            .map_err(|e: String| AppError::Validation(e))?;
        account.name = name.to_string();
        account.kind = kind;
        account.currency = currency.to_string();
        account.is_active = is_active;
        self.repo.update(&account)?;
        Ok(())
    }

    /// 删除账户
    pub fn delete(&self, id: i64, force: bool) -> AppResult<()> {
        if !force {
            return Err(AppError::Validation(
                "删除账户需要 --force 确认，此操作不可逆且会级联删除关联交易".into(),
            ));
        }
        if !self.repo.delete(id)? {
            return Err(AppError::NotFound(format!("账户 ID={id} 不存在")));
        }
        Ok(())
    }

    /// 净资产：所有活跃账户余额之和
    pub fn net_worth(&self) -> AppResult<i64> {
        let accounts = self.repo.find_active()?;
        Ok(accounts.iter().map(|a| a.balance_cents).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_account_lifecycle() {
        let db = Database::open_memory().unwrap();
        let svc = AccountService::new(&db);

        // Create
        let id = svc.create("工资卡", "bank_card", "CNY", 1000.0).unwrap();
        assert!(id > 0);

        // Get
        let acc = svc.get(id).unwrap();
        assert_eq!(acc.name, "工资卡");
        assert_eq!(acc.balance_cents, 100_000);

        // List
        let list = svc.list(false).unwrap();
        assert_eq!(list.len(), 1);

        // Delete with force
        assert!(svc.delete(id, false).is_err()); // need force
        svc.delete(id, true).unwrap();
        assert!(svc.get(id).is_err());
    }
}
