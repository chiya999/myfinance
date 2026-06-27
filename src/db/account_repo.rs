use rusqlite::Connection;

use super::Repository;
use crate::error::{AppError, AppResult};
use crate::models::{Account, AccountKind};

/// 账户仓库
pub struct AccountRepository<'a> {
    conn: &'a Connection,
}

impl<'a> AccountRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> Repository<Account> for AccountRepository<'a> {
    fn create(&self, account: &Account) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO accounts (name, kind, currency, balance_cents, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                account.name,
                account.kind.to_db(),
                account.currency,
                account.balance_cents,
                account.is_active as i32,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_by_id(&self, id: i64) -> AppResult<Option<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, currency, balance_cents, is_active, created_at, updated_at FROM accounts WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Account {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: AccountKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(AccountKind::Wallet),
                currency: row.get(3)?,
                balance_cents: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> AppResult<Vec<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, currency, balance_cents, is_active, created_at, updated_at FROM accounts ORDER BY is_active DESC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Account {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: AccountKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(AccountKind::Wallet),
                currency: row.get(3)?,
                balance_cents: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }

    fn update(&self, account: &Account) -> AppResult<bool> {
        let affected = self.conn.execute(
            "UPDATE accounts SET name = ?1, kind = ?2, currency = ?3, is_active = ?4, balance_cents = ?5, updated_at = datetime('now','localtime') WHERE id = ?6",
            rusqlite::params![
                account.name,
                account.kind.to_db(),
                account.currency,
                account.is_active as i32,
                account.balance_cents,
                account.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: i64) -> AppResult<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM accounts WHERE id = ?1", [id])?;
        Ok(affected > 0)
    }
}

impl AccountRepository<'_> {
    /// 只查询活跃账户
    pub fn find_active(&self) -> AppResult<Vec<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, currency, balance_cents, is_active, created_at, updated_at FROM accounts WHERE is_active = 1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Account {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: AccountKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(AccountKind::Wallet),
                currency: row.get(3)?,
                balance_cents: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }

    /// 直接更新余额（用于交易冲正等内部操作）
    pub fn update_balance(&self, account_id: i64, new_balance_cents: i64) -> AppResult<()> {
        self.conn.execute(
            "UPDATE accounts SET balance_cents = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
            rusqlite::params![new_balance_cents, account_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_account_crud() {
        let db = Database::open_memory().unwrap();
        let repo = AccountRepository::new(db.conn());

        // Create
        let account = Account::new("工资卡", AccountKind::BankCard, "CNY", 100000);
        let id = repo.create(&account).unwrap();
        assert!(id > 0);

        // Find
        let found = repo.find_by_id(id).unwrap().unwrap();
        assert_eq!(found.name, "工资卡");
        assert_eq!(found.balance_cents, 100000);

        // Update
        let mut updated = found;
        updated.name = "储蓄卡".to_string();
        updated.balance_cents = 200000;
        assert!(repo.update(&updated).unwrap());

        let after = repo.find_by_id(id).unwrap().unwrap();
        assert_eq!(after.name, "储蓄卡");
        assert_eq!(after.balance_cents, 200000);

        // Delete
        assert!(repo.delete(id).unwrap());
        assert!(repo.find_by_id(id).unwrap().is_none());
    }
}
