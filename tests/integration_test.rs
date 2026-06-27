/// 端到端集成测试
use chrono::Datelike;
use rust_course::db::{AccountRepository, Database, Repository};
use rust_course::engine::{AccountService, BudgetService, ReportService, TxnService};
use rust_course::models::*;

/// 获取本月第 N 天的日期字符串
fn this_month_day(day: u32) -> String {
    let today = chrono::Local::now().date_naive();
    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), day)
        .unwrap()
        .to_string()
}

#[test]
fn test_full_lifecycle() {
    let db = Database::open_memory().unwrap();

    // 创建账户
    let acc_svc = AccountService::new(&db);
    let acc1 = acc_svc
        .create("招行工资卡", "bank_card", "CNY", 10000.0)
        .unwrap();
    let acc2 = acc_svc.create("现金", "wallet", "CNY", 500.0).unwrap();
    assert_eq!(acc_svc.list(false).unwrap().len(), 2);

    // 记账（使用当月日期）
    let txn_svc = TxnService::new(&db);
    txn_svc
        .create(
            acc1,
            12,
            15000.0,
            "income",
            "工资",
            Some(&this_month_day(1)),
        )
        .unwrap(); // 分类12=工资
    txn_svc
        .create(acc1, 5, 35.0, "expense", "午餐", Some(&this_month_day(2)))
        .unwrap(); // 分类5=餐饮
    txn_svc
        .create(acc1, 9, 88.0, "expense", "电影", Some(&this_month_day(5)))
        .unwrap(); // 分类9=娱乐

    // 转账
    txn_svc
        .transfer(acc1, acc2, 2000.0, "取现", Some(&this_month_day(10)))
        .unwrap();

    // 验证余额
    let from_acc = acc_svc.get(acc1).unwrap();
    let to_acc = acc_svc.get(acc2).unwrap();
    // 10000 + 15000 - 35 - 88 - 2000 = 22877
    assert_eq!(from_acc.balance_cents, 2_287_700);
    assert_eq!(to_acc.balance_cents, 250_000);

    // 预算 — 餐饮 ¥1000, 娱乐 ¥500
    let budget_svc = BudgetService::new(&db);
    budget_svc.set(5, 1000.0, "monthly").unwrap();
    budget_svc.set(9, 500.0, "monthly").unwrap();

    let status = budget_svc.status().unwrap();
    let food = status.iter().find(|s| s.category_id == 5).unwrap();
    assert_eq!(food.spent_cents, 3_500); // ¥35
    assert!(food.progress_pct < 80.0);

    // 超支告警（阈值 3%）
    let alerts = budget_svc.alerts(3.0).unwrap();
    assert!(alerts.iter().any(|s| s.category_id == 5));

    // 报表
    let rpt_svc = ReportService::new(&db);
    let today = chrono::Local::now().date_naive();
    let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let summary = rpt_svc.summary(month_start, today).unwrap();
    assert_eq!(summary.total_income, 1_500_000);

    // 删除交易并冲正
    let txns = txn_svc
        .list(Some(acc1), None, None, None, None, None, None, 50)
        .unwrap();
    let income_txn = txns
        .iter()
        .find(|t| matches!(t.txn_type, TxnType::Income))
        .unwrap();
    let balance_before = acc_svc.get(acc1).unwrap().balance_cents;
    txn_svc.delete(income_txn.id.unwrap()).unwrap();
    let balance_after = acc_svc.get(acc1).unwrap().balance_cents;
    assert_eq!(balance_after, balance_before - income_txn.amount_cents);
}

#[test]
fn test_category_defaults() {
    let db = Database::open_memory().unwrap();
    let categories = rust_course::db::CategoryRepository::new(db.conn())
        .find_all()
        .unwrap();
    assert!(categories.len() >= 18);
    assert!(categories.iter().any(|c| c.name == "餐饮"));
    assert!(categories.iter().any(|c| c.name == "工资"));
}

#[test]
fn test_edge_cases() {
    let db = Database::open_memory().unwrap();
    let acc_svc = AccountService::new(&db);
    let txn_svc = TxnService::new(&db);
    let budget_svc = BudgetService::new(&db);

    let acc = acc_svc.create("测试卡", "bank_card", "CNY", 100.0).unwrap();
    assert!(acc_svc.delete(acc, false).is_err());
    assert!(acc_svc.delete(acc, true).is_ok());

    // 不能向同一账户转账
    let acc2 = acc_svc.create("卡2", "wallet", "CNY", 0.0).unwrap();
    assert!(txn_svc.transfer(acc2, acc2, 100.0, "自转", None).is_err());

    // 不能为收入分类设预算（分类12 = 工资，是收入分类）
    assert!(budget_svc.set(12, 100.0, "monthly").is_err());
}
