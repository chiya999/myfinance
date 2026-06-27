use rusqlite::Connection;

use super::Repository;
use crate::error::{AppError, AppResult};
use crate::models::{Category, CategoryKind};

/// 分类仓库
pub struct CategoryRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CategoryRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 按类别（支出/收入）查询
    pub fn find_by_kind(&self, kind: CategoryKind) -> AppResult<Vec<Category>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, icon, parent_id, sort_order, is_default FROM categories WHERE kind = ?1 ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map([kind.to_db()], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: CategoryKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(CategoryKind::Expense),
                icon: row.get::<_, String>(3).unwrap_or_default(),
                parent_id: row.get(4)?,
                sort_order: row.get(5)?,
                is_default: row.get::<_, i32>(6)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }

    /// 查询所有分类（保持排序）
    pub fn find_all_sorted(&self) -> AppResult<Vec<Category>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, icon, parent_id, sort_order, is_default FROM categories ORDER BY kind DESC, sort_order ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: CategoryKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(CategoryKind::Expense),
                icon: row.get::<_, String>(3).unwrap_or_default(),
                parent_id: row.get(4)?,
                sort_order: row.get(5)?,
                is_default: row.get::<_, i32>(6)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(AppError::from)
    }
}

impl<'a> Repository<Category> for CategoryRepository<'a> {
    fn create(&self, category: &Category) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO categories (name, kind, icon, parent_id, sort_order, is_default) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                category.name,
                category.kind.to_db(),
                category.icon,
                category.parent_id,
                category.sort_order,
                category.is_default as i32,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_by_id(&self, id: i64) -> AppResult<Option<Category>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, icon, parent_id, sort_order, is_default FROM categories WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                kind: CategoryKind::from_db(&row.get::<_, String>(2)?)
                    .unwrap_or(CategoryKind::Expense),
                icon: row.get::<_, String>(3).unwrap_or_default(),
                parent_id: row.get(4)?,
                sort_order: row.get(5)?,
                is_default: row.get::<_, i32>(6)? != 0,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> AppResult<Vec<Category>> {
        self.find_all_sorted()
    }

    fn update(&self, category: &Category) -> AppResult<bool> {
        let affected = self.conn.execute(
            "UPDATE categories SET name = ?1, kind = ?2, icon = ?3, parent_id = ?4, sort_order = ?5, is_default = ?6 WHERE id = ?7",
            rusqlite::params![
                category.name,
                category.kind.to_db(),
                category.icon,
                category.parent_id,
                category.sort_order,
                category.is_default as i32,
                category.id,
            ],
        )?;
        Ok(affected > 0)
    }

    fn delete(&self, id: i64) -> AppResult<bool> {
        let affected = self.conn.execute(
            "DELETE FROM categories WHERE id = ?1 AND is_default = 0",
            [id],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_default_categories_exist() {
        let db = Database::open_memory().unwrap();
        let repo = CategoryRepository::new(db.conn());

        let all = repo.find_all().unwrap();
        assert!(all.len() >= 18);

        let expenses = repo.find_by_kind(CategoryKind::Expense).unwrap();
        let incomes = repo.find_by_kind(CategoryKind::Income).unwrap();
        assert!(expenses.len() >= 11);
        assert!(incomes.len() >= 7);
    }

    #[test]
    fn test_cannot_delete_default() {
        let db = Database::open_memory().unwrap();
        let repo = CategoryRepository::new(db.conn());
        let all = repo.find_all().unwrap();
        let default_cat = all.iter().find(|c| c.is_default).unwrap();

        // 预置分类不允许删除
        let result = repo.delete(default_cat.id.unwrap()).unwrap();
        assert!(!result);
    }
}
