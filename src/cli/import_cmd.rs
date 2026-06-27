use clap::Subcommand;
use std::path::PathBuf;

use crate::db::Database;
use crate::engine::TxnService;
use crate::error::AppResult;
use crate::import;

#[derive(Subcommand)]
pub enum ImportCommand {
    /// 导入支付宝账单
    Alipay {
        file: PathBuf,
        #[arg(short, long)]
        account: Option<i64>,
    },
    /// 导入微信账单
    Wechat {
        file: PathBuf,
        #[arg(short, long)]
        account: Option<i64>,
    },
    /// 自动检测格式并导入
    Auto {
        file: PathBuf,
        #[arg(short, long)]
        account: Option<i64>,
    },
}

pub fn handle(cmd: ImportCommand, db: &Database) -> AppResult<()> {
    match cmd {
        ImportCommand::Alipay { file, account } => {
            println!("📥 导入支付宝账单: {}", file.display());
            let result = import::import_file(&file)?;
            commit_import(db, &result, account)?;
        }
        ImportCommand::Wechat { file, account } => {
            println!("📥 导入微信账单: {}", file.display());
            let result = import::import_file(&file)?;
            commit_import(db, &result, account)?;
        }
        ImportCommand::Auto { file, account } => {
            println!("📥 自动检测导入: {}", file.display());
            let result = import::import_file(&file)?;
            println!("   识别格式: {}", result.format_name);
            commit_import(db, &result, account)?;
        }
    }
    Ok(())
}

fn commit_import(
    db: &Database,
    result: &import::ImportResult,
    account_id: Option<i64>,
) -> AppResult<()> {
    if result.transactions.is_empty() {
        println!("   没有可导入的交易记录");
        return Ok(());
    }

    // 如果没有指定账户，尝试用第一个活跃账户
    let account_id = match account_id {
        Some(id) => id,
        None => {
            let accounts = crate::db::AccountRepository::new(db.conn()).find_active()?;
            if accounts.is_empty() {
                return Err(crate::error::AppError::Validation(
                    "没有可用的活跃账户，请先用 account add 创建，或使用 --account 指定".into(),
                ));
            }
            accounts[0].id.unwrap()
        }
    };

    let txn_svc = TxnService::new(db);
    let mut success = 0;
    let mut skipped = 0;

    for txn in &result.transactions {
        // 尝试自动匹配分类 — 简化为使用分类 ID 1（工资）作为收入，5（餐饮）作为默认支出
        let category_id = if txn.txn_type == "income" { 1 } else { 5 };

        match txn_svc.create(
            account_id,
            category_id,
            txn.amount_cents as f64 / 100.0,
            &txn.txn_type,
            &txn.description,
            Some(&txn.txn_date.to_string()),
        ) {
            Ok(_) => success += 1,
            Err(e) => {
                skipped += 1;
                eprintln!("   跳过: {e}");
            }
        }
    }

    println!("✅ 导入完成: {success} 条成功, {skipped} 条跳过");

    if !result.warnings.is_empty() {
        println!("⚠️  警告:");
        for w in &result.warnings {
            println!("   - {w}");
        }
    }

    Ok(())
}
