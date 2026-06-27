use chrono::NaiveDate;
use clap::Subcommand;
use std::str::FromStr;

use super::filter::TxnFilters;
use crate::db::Database;
use crate::engine::TxnService;
use crate::error::AppResult;
use crate::models::{TxnType, cents_to_yuan};

#[derive(Subcommand)]
pub enum TxnCommand {
    /// 列出交易记录
    List {
        #[command(flatten)]
        filters: TxnFilters,
    },
    /// 添加收入/支出
    Add {
        account_id: i64,
        category_id: i64,
        amount: f64,
        txn_type: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short = 'D', long)]
        date: Option<String>,
    },
    /// 账户间转账
    Transfer {
        from_account: i64,
        to_account: i64,
        amount: f64,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short = 'D', long)]
        date: Option<String>,
    },
    /// 删除交易
    Delete { id: i64 },
}

pub fn handle(cmd: TxnCommand, db: &Database) -> AppResult<()> {
    let svc = TxnService::new(db);

    match cmd {
        TxnCommand::List { filters } => {
            let txn_type = filters
                .txn_type
                .as_deref()
                .and_then(|s| TxnType::from_str(s).ok());
            let from_date = filters
                .from_date
                .as_deref()
                .map(|s| NaiveDate::from_str(s).ok())
                .flatten();
            let to_date = filters
                .to_date
                .as_deref()
                .map(|s| NaiveDate::from_str(s).ok())
                .flatten();
            let min_amount = filters.min_amount.map(|y| (y * 100.0) as i64);
            let max_amount = filters.max_amount.map(|y| (y * 100.0) as i64);

            let txns = svc.list(
                filters.account_id,
                filters.category_id,
                txn_type,
                from_date,
                to_date,
                min_amount,
                max_amount,
                filters.limit,
            )?;

            println!("📋 交易列表 ({} 条)\n", txns.len());
            if txns.is_empty() {
                println!("  (暂无记录)");
                return Ok(());
            }
            println!(
                "{:<6} {:<12} {:<8} {:<8} {:<12} {:<20}",
                "ID", "日期", "类型", "分类ID", "金额", "描述"
            );
            println!("{}", "─".repeat(72));
            for t in &txns {
                let sign = match t.txn_type {
                    TxnType::Income => "+",
                    TxnType::Expense => "-",
                    TxnType::Transfer => "↔",
                };
                println!(
                    "{:<6} {:<12} {:<8} {:<8} {}{:<11} {:<20}",
                    t.id.unwrap(),
                    t.txn_date.to_string(),
                    t.txn_type.display_name(),
                    t.category_id.map_or("-".to_string(), |c| c.to_string()),
                    sign,
                    cents_to_yuan(t.amount_cents),
                    if t.description.len() > 18 {
                        format!("{}...", &t.description[..18])
                    } else {
                        t.description.clone()
                    },
                );
            }
        }
        TxnCommand::Add {
            account_id,
            category_id,
            amount,
            txn_type,
            description,
            date,
        } => {
            let id = svc.create(
                account_id,
                category_id,
                amount,
                &txn_type,
                description.as_deref().unwrap_or(""),
                date.as_deref(),
            )?;
            println!("✅ 交易创建成功: ID={id}");
        }
        TxnCommand::Transfer {
            from_account,
            to_account,
            amount,
            description,
            date,
        } => {
            let id = svc.transfer(
                from_account,
                to_account,
                amount,
                description.as_deref().unwrap_or(""),
                date.as_deref(),
            )?;
            println!("✅ 转账成功: ID={id}");
        }
        TxnCommand::Delete { id } => {
            svc.delete(id)?;
            println!("✅ 交易 ID={id} 已删除，余额已冲正");
        }
    }
    Ok(())
}
