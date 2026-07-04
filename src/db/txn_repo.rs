use chrono::NaiveDate;
use rusqlite::Connection;

use super::Repository;
use crate::error::{AppError, AppResult};
use crate::models::{Transaction, TxnType};

/// 交易仓库
pub struct TransactionRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TransactionRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 按条件筛选交易列表
    #[allow(clippy::too_many_arguments)]
    pub fn find_filtered(
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
        let mut sql = String::from(
            "SELECT id, account_id, category_id, type, amount_cents, to_account_id, description, txn_date, created_at FROM transactions WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(aid) = account_id {
            sql.push_str(" AND account_id = ?");
            params.push(Box::new(aid));
        }
        if let Some(cid) = category_id {
            sql.push_str(" AND category_id = ?");
            params.push(Box::new(cid));
        }
        if let Some(tt) = txn_type {
            sql.push_str(" AND type = ?");
            params.push(Box::new(tt.to_db().to_string()));
        }
        if let Some(from) = from_date {
            sql.push_str(" AND txn_date >= ?");
            params.push(Box::new(from.to_string()));
        }
        if let Some(to) = to_date {
            sql.push_str(" AND txn_date <= ?");
            params.push(Box::new(to.to_string()));
        }
        if let Some(min) = min_amount {
            sql.push_str(" AND amount_cents >= ?");
            params.push(Box::new(min));
        }
        if let Some(max) = max_amount {
            sql.push_str(" AND amount_cents <= ?");
            params.push(Box::new(max));
        }

        sql.push_str(&format!(
            " ORDER BY txn_date DESC, id DESC LIMIT {}",
            limit.min(1000)
        ));

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let type_str: String = row.get(3)?;
            Ok(Transaction {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                category_id: row.get(2)?,
                txn_type: TxnType::from_db(&type_str).unwrap_or(TxnType::Expense),
                amount_cents: row.get(4)?,
                to_account_id: row.get(5)?,
                description: row.get(6)?,
                txn_date: NaiveDate::parse_from_str(&row.get::<_, String>(7)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                created_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }
}

impl<'a> Repository<Transaction> for TransactionRepository<'a> {
    fn create(&self, txn: &Transaction) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO transactions (account_id, category_id, type, amount_cents, to_account_id, description, txn_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                txn.account_id,
                txn.category_id,
                txn.txn_type.to_db(),
                txn.amount_cents,
                txn.to_account_id,
                txn.description,
                txn.txn_date.to_string(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_by_id(&self, id: i64) -> AppResult<Option<Transaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, category_id, type, amount_cents, to_account_id, description, txn_date, created_at FROM transactions WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let type_str: String = row.get(3)?;
            Ok(Transaction {
                id: Some(row.get(0)?),
                account_id: row.get(1)?,
                category_id: row.get(2)?,
                txn_type: TxnType::from_db(&type_str).unwrap_or(TxnType::Expense),
                amount_cents: row.get(4)?,
                to_account_id: row.get(5)?,
                description: row.get(6)?,
                txn_date: NaiveDate::parse_from_str(&row.get::<_, String>(7)?, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                created_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> AppResult<Vec<Transaction>> {
        self.find_filtered(None, None, None, None, None, None, None, 100)
    }

    fn update(&self, txn: &Transaction) -> AppResult<bool> {
        let affected = self.conn.execute(
            "UPDATE transactions SET account_id = ?1, category_id = ?2, type = ?3, amount_cents = ?4, to_account_id = ?5, description = ?6, txn_date = ?7 WHERE id = ?8",
            rusqlite::params![
                txn.account_id,
                txn.category_id,
                txn.txn_type.to_db(),
                txn.amount_cents,
                txn.to_account_id,
                txn.description,
                txn.txn_date.to_string(),
                txn.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: i64) -> AppResult<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM transactions WHERE id = ?1", [id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{AccountRepository, Database};
    use crate::models::Account;

    #[test]
    fn test_transaction_create_and_find() {
        let db = Database::open_memory().unwrap();
        let acc_repo = AccountRepository::new(db.conn());
        let txn_repo = TransactionRepository::new(db.conn());

        let acc = Account::new("测试卡", crate::models::AccountKind::BankCard, "CNY", 0);
        let account_id = acc_repo.create(&acc).unwrap();

        let txn = Transaction::new_income_expense(
            account_id,
            1, // 工资
            TxnType::Income,
            500000,
            "测试收入",
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
        );
        let txn_id = txn_repo.create(&txn).unwrap();
        assert!(txn_id > 0);

        let found = txn_repo.find_by_id(txn_id).unwrap().unwrap();
        assert_eq!(found.amount_cents, 500000);
        assert_eq!(found.txn_type, TxnType::Income);
    }

    #[test]
    fn test_filtered_query() {
        let db = Database::open_memory().unwrap();
        let acc_repo = AccountRepository::new(db.conn());
        let txn_repo = TransactionRepository::new(db.conn());

        let acc = Account::new("测试卡", crate::models::AccountKind::BankCard, "CNY", 0);
        let account_id = acc_repo.create(&acc).unwrap();

        // 插入两笔不同日期的交易
        txn_repo
            .create(&Transaction::new_income_expense(
                account_id,
                1,
                TxnType::Income,
                100000,
                "6月收入",
                NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            ))
            .unwrap();
        txn_repo
            .create(&Transaction::new_income_expense(
                account_id,
                1,
                TxnType::Income,
                200000,
                "7月收入",
                NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
            ))
            .unwrap();

        // 筛选 6 月
        let jun = txn_repo
            .find_filtered(
                None,
                None,
                None,
                Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()),
                None,
                None,
                50,
            )
            .unwrap();
        assert_eq!(jun.len(), 1);
        assert_eq!(jun[0].amount_cents, 100000);
    }
}
