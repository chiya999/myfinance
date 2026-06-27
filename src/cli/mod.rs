use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::error::AppResult;

mod account_cmd;
mod budget_cmd;
mod filter;
mod import_cmd;
mod report_cmd;
mod txn_cmd;

// TxnFilters is used via filter:: qualified path in subcommands

/// 终端个人财务管理器
#[derive(Parser)]
#[command(
    name = "myfinance",
    version,
    about = "Terminal Personal Finance Manager"
)]
pub struct Cli {
    /// SQLite 数据库文件路径 [env: MYFINANCE_DB]
    #[arg(long, default_value = "myfinance.db")]
    pub database: PathBuf,

    /// 启用详细日志
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// 启动交互式 TUI 界面
    Tui,

    /// 管理账户
    #[command(subcommand, name = "account")]
    Account(account_cmd::AccountCommand),

    /// 管理交易
    #[command(subcommand, name = "txn")]
    Txn(txn_cmd::TxnCommand),

    /// 管理预算
    #[command(subcommand, name = "budget")]
    Budget(budget_cmd::BudgetCommand),

    /// 生成报表
    #[command(subcommand, name = "report")]
    Report(report_cmd::ReportCommand),

    /// 导入外部账单
    #[command(subcommand, name = "import")]
    Import(import_cmd::ImportCommand),
}

impl Cli {
    /// 分发命令到对应的 handler
    pub fn dispatch(self) -> AppResult<()> {
        use crate::db::Database;

        let db = if self.database.to_str() == Some(":memory:") {
            Database::open_memory()?
        } else {
            Database::open(&self.database)?
        };

        match self.command {
            None | Some(Command::Tui) => crate::tui::run_tui(db),
            Some(Command::Account(cmd)) => account_cmd::handle(cmd, &db),
            Some(Command::Txn(cmd)) => txn_cmd::handle(cmd, &db),
            Some(Command::Budget(cmd)) => budget_cmd::handle(cmd, &db),
            Some(Command::Report(cmd)) => report_cmd::handle(cmd, &db),
            Some(Command::Import(cmd)) => import_cmd::handle(cmd, &db),
        }
    }
}
