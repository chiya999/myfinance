use rusqlite::Connection;

use crate::error::AppResult;

/// 运行所有迁移脚本
pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    // 使用 user_version PRAGMA 跟踪已应用的迁移版本
    let current_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version < 1 {
        let sql = include_str!("../../migrations/001_init.sql");
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", 1)?;
    }

    // 未来迁移在此叠加:
    // if current_version < 2 { ... }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run_migrations(&conn).unwrap();

        // 验证四个核心表存在
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"accounts".to_string()));
        assert!(tables.contains(&"categories".to_string()));
        assert!(tables.contains(&"transactions".to_string()));
        assert!(tables.contains(&"budgets".to_string()));

        // 验证预置分类数量（11 支出 + 7 收入 = 18）
        let cat_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .unwrap();
        assert!(
            cat_count >= 18,
            "Expected >= 18 default categories, got {cat_count}"
        );
    }

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        // 运行两次迁移不应报错
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn test_migration_from_zero_version() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        // version 默认为 0，迁移应该执行
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, 0);
        run_migrations(&conn).unwrap();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }
}
