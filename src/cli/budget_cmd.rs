use clap::Subcommand;

use crate::db::Database;
use crate::engine::BudgetService;
use crate::error::AppResult;
use crate::models::cents_to_yuan;

#[derive(Subcommand)]
pub enum BudgetCommand {
    /// 列出所有预算
    List,
    /// 设置分类预算
    Set {
        category_id: i64,
        amount: f64,
        #[arg(short, long, default_value = "monthly")]
        period: String,
    },
    /// 查看预算执行状态
    Status,
    /// 查看超支告警
    Alert {
        #[arg(short, long, default_value = "80")]
        threshold: f64,
    },
}

pub fn handle(cmd: BudgetCommand, db: &Database) -> AppResult<()> {
    let svc = BudgetService::new(db);

    match cmd {
        BudgetCommand::List => {
            let budgets = svc.list()?;
            println!("📋 预算列表\n");
            if budgets.is_empty() {
                println!("  (暂无预算，使用 budget set 创建)");
            } else {
                println!(
                    "{:<6} {:<10} {:<14} {:<12} {:<12}",
                    "ID", "分类ID", "金额", "周期", "开始日期"
                );
                println!("{}", "─".repeat(58));
                for b in &budgets {
                    println!(
                        "{:<6} {:<10} ¥{:<13} {:<12} {}",
                        b.id.unwrap(),
                        b.category_id,
                        cents_to_yuan(b.amount_cents),
                        if b.period == crate::models::BudgetPeriod::Monthly {
                            "月度"
                        } else {
                            "周度"
                        },
                        b.start_date,
                    );
                }
            }
        }
        BudgetCommand::Set {
            category_id,
            amount,
            period,
        } => {
            let id = svc.set(category_id, amount, &period)?;
            println!("✅ 预算设置成功: ID={id}");
        }
        BudgetCommand::Status => {
            let statuses = svc.status()?;
            println!("📊 预算执行状态\n");
            if statuses.is_empty() {
                println!("  (暂无有效预算)");
                return Ok(());
            }
            println!(
                "{:<12} {:<10} {:<12} {:<12} {:>8}",
                "分类", "预算", "已花", "剩余", "进度"
            );
            println!("{}", "─".repeat(62));
            for s in &statuses {
                let bar = progress_bar(s.progress_pct, 20);
                println!(
                    "{}{:<10} ¥{:<11} ¥{:<11} ¥{:<11} {:>6.1}% {}",
                    s.category_icon,
                    s.category_name,
                    cents_to_yuan(s.amount_cents),
                    cents_to_yuan(s.spent_cents),
                    cents_to_yuan(s.remaining_cents),
                    s.progress_pct,
                    bar,
                );
            }
        }
        BudgetCommand::Alert { threshold } => {
            let alerts = svc.alerts(threshold)?;
            println!("🚨 超支告警 (阈值: {threshold}%)\n");
            if alerts.is_empty() {
                println!("  ✅ 所有预算均在安全范围内");
                return Ok(());
            }
            for s in &alerts {
                let emoji = if s.progress_pct >= 100.0 {
                    "🔴"
                } else {
                    "🟡"
                };
                println!(
                    "  {} {}: 已花 ¥{} / 预算 ¥{} ({:.1}%)",
                    emoji,
                    s.category_name,
                    cents_to_yuan(s.spent_cents),
                    cents_to_yuan(s.amount_cents),
                    s.progress_pct,
                );
            }
        }
    }
    Ok(())
}

fn progress_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0 * width as f64).round() as usize).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
