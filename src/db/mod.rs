mod connection;
mod migrations;

// Repository 模块
mod account_repo;
mod budget_repo;
mod category_repo;
mod txn_repo;

pub use connection::Database;

pub use account_repo::AccountRepository;
pub use budget_repo::BudgetRepository;
pub use category_repo::CategoryRepository;
pub use txn_repo::TransactionRepository;

use crate::error::AppResult;

/// 通用 Repository trait —— 定义标准 CRUD 操作
pub trait Repository<T> {
    /// 创建记录，返回新 ID
    fn create(&self, item: &T) -> AppResult<i64>;
    /// 按 ID 查找
    fn find_by_id(&self, id: i64) -> AppResult<Option<T>>;
    /// 查询全部
    fn find_all(&self) -> AppResult<Vec<T>>;
    /// 更新记录，返回是否成功（行数 > 0）
    fn update(&self, item: &T) -> AppResult<bool>;
    /// 删除记录
    fn delete(&self, id: i64) -> AppResult<bool>;
}
