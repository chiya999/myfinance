use rusqlite::Connection;
use std::path::Path;

use super::migrations::run_migrations;
use crate::error::AppResult;

/// 数据库句柄 —— 持有 SQLite 连接
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 打开（或创建）磁盘数据库文件并运行迁移
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        // 启用外键约束
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// 打开内存数据库（仅测试用）
    pub fn open_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// 获取内部连接的只读引用
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// 获取内部连接的可变引用（用于事务操作）
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
