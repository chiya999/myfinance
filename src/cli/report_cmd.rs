use chrono::{Datelike, NaiveDate};
use clap::Subcommand;
use std::str::FromStr;

use std::path::PathBuf;

use crate::db::{Database, Repository, TransactionRepository};
use crate::engine::{ReportService, TrendGranularity};
use crate::error::AppResult;
use crate::export::{self, ExportData};
use crate::models::{CategoryKind, cents_to_yuan};

#[derive(Subcommand)]
pub enum ReportCommand {
    /// 月度收支总览
    Summary {
        /// 起始日期 (YYYY-MM-DD)，默认当年1月1日
        #[arg(long)]
        from: Option<String>,
        /// 结束日期 (YYYY-MM-DD)，默认今天
        #[arg(long)]
        to: Option<String>,
    },
    /// 按分类统计
    Category {
        #[arg(short, long, default_value = "expense")]
        kind: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "table")]
        chart: String,
    },
    /// 收支趋势
    Trend {
        #[arg(short, long, default_value = "monthly")]
        granularity: String,
        #[arg(short, long, default_value = "6")]
        months: u32,
    },
    /// 账户快照
    AccountSnapshot,
    /// 导出数据为 CSV 或 JSON
    Export {
        /// 导出格式: csv 或 json
        #[arg(short, long, default_value = "csv")]
        format: String,
        /// 输出文件路径
        #[arg(short, long, default_value = "myfinance_export.csv")]
        output: PathBuf,
    },
}

pub fn handle(cmd: ReportCommand, db: &Database) -> AppResult<()> {
    let svc = ReportService::new(db);

    match cmd {
        ReportCommand::Summary { from, to } => {
            let today = chrono::Local::now().date_naive();
            let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            let from = from.as_deref().map(parse_date).transpose()?.unwrap_or(year_start);
            let to = to.as_deref().map(parse_date).transpose()?.unwrap_or(today);
            let r = svc.summary(from, to)?;

            println!("📊 收支总览: {} ~ {}\n", r.period_start, r.period_end);
            println!("{}", "─".repeat(40));
            println!("  总收入:   ¥{}", cents_to_yuan(r.total_income));
            println!("  总支出:   ¥{}", cents_to_yuan(r.total_expense));
            println!("  净收支:   ¥{}", cents_to_yuan(r.net));
            println!("  交易数:   {}", r.transaction_count);
            println!("{}", "─".repeat(40));
        }
        ReportCommand::Category {
            kind,
            from,
            to,
            chart: _chart,
        } => {
            let cat_kind = match kind.as_str() {
                "income" | "收入" => CategoryKind::Income,
                _ => CategoryKind::Expense,
            };
            let today = chrono::Local::now().date_naive();
            let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            let from = from.as_deref().map(parse_date).transpose()?.unwrap_or(year_start);
            let to = to.as_deref().map(parse_date).transpose()?.unwrap_or(today);

            let breakdown = svc.category_breakdown(cat_kind, from, to)?;
            println!(
                "📊 分类统计 ({}) {} ~ {}\n",
                cat_kind.display_name(),
                from,
                to
            );
            println!("{:<14} {:<12} {:>7}", "分类", "金额", "占比");
            println!("{}", "─".repeat(38));
            for b in &breakdown {
                println!(
                    "{}{:<11} ¥{:<11} {:>5.1}%",
                    b.category_icon,
                    format!("[{}]", b.category_name),
                    cents_to_yuan(b.amount_cents),
                    b.percentage,
                );
            }
        }
        ReportCommand::Trend {
            granularity,
            months,
        } => {
            let to = chrono::Local::now().date_naive();
            let from = to
                .checked_sub_months(chrono::Months::new(months))
                .unwrap_or(to);
            let gran = match granularity.as_str() {
                "daily" | "日" => TrendGranularity::Daily,
                "weekly" | "周" => TrendGranularity::Weekly,
                _ => TrendGranularity::Monthly,
            };

            let trend = svc.trend(from, to, gran)?;
            println!("📈 收支趋势 ({months} 个月)\n");
            for t in &trend {
                let bar_len = (t.expense_cents.max(1) as f64 / 1000.0).max(1.0).min(40.0) as usize;
                println!(
                    "  {}  +¥{}  -¥{}  {}",
                    t.date,
                    cents_to_yuan(t.income_cents),
                    cents_to_yuan(t.expense_cents),
                    "█".repeat(bar_len),
                );
            }
        }
        ReportCommand::Export { format, output } => {
            let txns = TransactionRepository::new(db.conn()).find_filtered(
                None, None, None, None, None, None, None, 10000,
            )?;
            let accounts = crate::db::AccountRepository::new(db.conn()).find_all()?;
            let categories = crate::db::CategoryRepository::new(db.conn()).find_all()?;
            let budgets = crate::db::BudgetRepository::new(db.conn()).find_all()?;
            let data = ExportData::new(accounts, txns, categories, budgets);
            export::export_to_file(&output, &format, &data)?;
        }
        ReportCommand::AccountSnapshot => {
            let snapshots = svc.account_snapshot()?;
            println!("💳 账户快照\n");
            println!(
                "{:<16} {:<10} {:<10} {:<10} {:<10}",
                "账户", "类型", "余额", "本月收入", "本月支出"
            );
            println!("{}", "─".repeat(60));
            for s in &snapshots {
                println!(
                    "{:<16} {:<10} ¥{:<9} ¥{:<9} ¥{:<9}",
                    s.account_name,
                    s.account_kind,
                    cents_to_yuan(s.balance_cents),
                    cents_to_yuan(s.month_income),
                    cents_to_yuan(s.month_expense),
                );
            }
        }
    }
    Ok(())
}

fn parse_date(s: &str) -> AppResult<NaiveDate> {
    NaiveDate::from_str(s).map_err(|_| {
        crate::error::AppError::Validation(format!("无效日期: {s}, 请使用 YYYY-MM-DD 格式"))
    })
}
