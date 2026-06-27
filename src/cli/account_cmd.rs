use clap::Subcommand;

use crate::db::Database;
use crate::engine::AccountService;
use crate::error::AppResult;
use crate::models::cents_to_yuan;

#[derive(Subcommand)]
pub enum AccountCommand {
    /// 列出所有账户
    List {
        #[arg(short, long)]
        show_inactive: bool,
    },
    /// 添加账户
    Add {
        name: String,
        #[arg(short, long)]
        kind: String,
        #[arg(short, long, default_value = "CNY")]
        currency: String,
        #[arg(short, long, default_value = "0")]
        initial_balance: f64,
    },
    /// 显示账户详情
    Show { id: i64 },
    /// 编辑账户 (基础版本 — 仅更改名称和状态)
    Edit {
        id: i64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        kind: Option<String>,
        #[arg(short, long)]
        currency: Option<String>,
        #[arg(short, long)]
        active: Option<bool>,
    },
    /// 删除账户
    Delete {
        id: i64,
        #[arg(short, long)]
        force: bool,
    },
}

pub fn handle(cmd: AccountCommand, db: &Database) -> AppResult<()> {
    let svc = AccountService::new(db);

    match cmd {
        AccountCommand::List { show_inactive } => {
            let accounts = svc.list(show_inactive)?;
            println!(
                "📋 账户列表 ({})\n",
                if show_inactive { "含停用" } else { "活跃" }
            );
            if accounts.is_empty() {
                println!("  (暂无账户，使用 account add 创建)");
            } else {
                println!(
                    "{:<6} {:<16} {:<10} {:<8} {:<10} 状态",
                    "ID", "名称", "类型", "币种", "余额"
                );
                println!("{}", "─".repeat(65));
                for a in &accounts {
                    let status = if a.is_active { "✅" } else { "⛔" };
                    println!(
                        "{:<6} {:<16} {:<10} {:<8} ¥{:<9} {}",
                        a.id.unwrap(),
                        a.name,
                        a.kind.display_name(),
                        a.currency,
                        cents_to_yuan(a.balance_cents),
                        status,
                    );
                }
            }
        }
        AccountCommand::Add {
            name,
            kind,
            currency,
            initial_balance,
        } => {
            let id = svc.create(&name, &kind, &currency, initial_balance)?;
            println!("✅ 账户创建成功: ID={id}");
        }
        AccountCommand::Show { id } => {
            let a = svc.get(id)?;
            println!("🔍 账户详情");
            println!("{}", "─".repeat(40));
            println!("  ID:     {}", a.id.unwrap());
            println!("  名称:   {}", a.name);
            println!("  类型:   {} ({})", a.kind.display_name(), a.kind.to_db());
            println!("  币种:   {}", a.currency);
            println!("  余额:   ¥{}", cents_to_yuan(a.balance_cents));
            println!("  状态:   {}", if a.is_active { "活跃" } else { "已停用" });
            println!("  创建:   {}", a.created_at.as_deref().unwrap_or("-"));
            println!("  更新:   {}", a.updated_at.as_deref().unwrap_or("-"));
        }
        AccountCommand::Edit {
            id,
            name,
            kind,
            currency,
            active,
        } => {
            let current = svc.get(id)?;
            let new_name = name.unwrap_or(current.name);
            let new_kind = kind.unwrap_or_else(|| current.kind.to_db().to_string());
            let new_currency = currency.unwrap_or(current.currency);
            let new_active = active.unwrap_or(current.is_active);
            svc.update(id, &new_name, &new_kind, &new_currency, new_active)?;
            println!("✅ 账户 ID={id} 更新成功");
        }
        AccountCommand::Delete { id, force } => {
            svc.delete(id, force)?;
            println!("✅ 账户 ID={id} 已删除");
        }
    }
    Ok(())
}
