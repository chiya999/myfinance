use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent};

use crate::db::Database;
use crate::engine::{AccountService, BudgetService, ReportService, TxnService};
use crate::models::cents_to_yuan;

/// TUI 标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Accounts,
    Transactions,
    Budget,
    Reports,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Dashboard,
        Tab::Accounts,
        Tab::Transactions,
        Tab::Budget,
        Tab::Reports,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Dashboard => "📊 概览",
            Tab::Accounts => "💳 账户",
            Tab::Transactions => "📝 交易",
            Tab::Budget => "🎯 预算",
            Tab::Reports => "📈 报表",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// 输入模式
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingTransaction,
    AddingAccount,
    AddingBudget,
    Help,
}

/// 应用主状态
pub struct App {
    pub db: Database,
    pub tab: Tab,
    pub mode: InputMode,
    pub should_quit: bool,

    // 表单临时状态
    pub input_buffer: String,
    pub input_fields: Vec<String>, // 多字段表单缓存
    pub current_field: usize,
    pub status_message: Option<String>,

    // 选中索引
    pub selected_index: usize,

    // 数据缓存（在 on_tick 中刷新）
    pub account_count: usize,
    pub total_balance_cents: i64,
    pub month_income: i64,
    pub month_expense: i64,
    pub recent_txns_summary: Vec<String>,
    pub budget_alerts: Vec<String>,
}

impl App {
    pub fn new(db: Database) -> Self {
        let mut app = Self {
            db,
            tab: Tab::Dashboard,
            mode: InputMode::Normal,
            should_quit: false,
            input_buffer: String::new(),
            input_fields: Vec::new(),
            current_field: 0,
            status_message: None,
            selected_index: 0,
            account_count: 0,
            total_balance_cents: 0,
            month_income: 0,
            month_expense: 0,
            recent_txns_summary: Vec::new(),
            budget_alerts: Vec::new(),
        };
        app.refresh_data();
        app
    }

    /// 刷新所有缓存数据
    pub fn refresh_data(&mut self) {
        let acc_svc = AccountService::new(&self.db);
        if let Ok(accounts) = acc_svc.list(false) {
            self.account_count = accounts.len();
            self.total_balance_cents = accounts.iter().map(|a| a.balance_cents).sum();
        }

        let today = chrono::Local::now().date_naive();
        let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let rpt_svc = ReportService::new(&self.db);
        if let Ok(summary) = rpt_svc.summary(month_start, today) {
            self.month_income = summary.total_income;
            self.month_expense = summary.total_expense;
        }

        // 最近 10 笔交易
        let txn_svc = TxnService::new(&self.db);
        if let Ok(txns) = txn_svc.list(None, None, None, None, None, None, None, 10) {
            self.recent_txns_summary = txns
                .iter()
                .map(|t| {
                    let sign = match t.txn_type {
                        crate::models::TxnType::Income => "+",
                        crate::models::TxnType::Expense => "-",
                        crate::models::TxnType::Transfer => "↔",
                    };
                    format!(
                        "{}  {}{:<12} {}",
                        t.txn_date,
                        sign,
                        cents_to_yuan(t.amount_cents),
                        t.description,
                    )
                })
                .collect();
        }

        // 预算告警
        let budget_svc = BudgetService::new(&self.db);
        if let Ok(alerts) = budget_svc.alerts(80.0) {
            self.budget_alerts = alerts
                .iter()
                .map(|s| {
                    format!(
                        "{} {}: {:.0}%",
                        s.category_icon, s.category_name, s.progress_pct,
                    )
                })
                .collect();
        }
    }

    /// 处理键盘事件，返回 false 表示退出
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Esc {
            if self.mode != InputMode::Normal {
                self.mode = InputMode::Normal;
                self.input_buffer.clear();
                self.input_fields.clear();
                return true;
            }
            self.should_quit = true;
            return false;
        }

        match &self.mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Help => {
                self.mode = InputMode::Normal;
                true
            }
            _ => {
                // 在表单模式下，按 Enter 提交
                if key.code == KeyCode::Enter {
                    self.submit_form();
                    return true;
                }
                if key.code == KeyCode::Backspace {
                    self.input_buffer.pop();
                    return true;
                }
                if let KeyCode::Char(ch) = key.code {
                    self.input_buffer.push(ch);
                    return true;
                }
                true
            }
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                false
            }
            KeyCode::Char('1') => {
                self.set_tab(Tab::Dashboard);
                true
            }
            KeyCode::Char('2') => {
                self.set_tab(Tab::Accounts);
                true
            }
            KeyCode::Char('3') => {
                self.set_tab(Tab::Transactions);
                true
            }
            KeyCode::Char('4') => {
                self.set_tab(Tab::Budget);
                true
            }
            KeyCode::Char('5') => {
                self.set_tab(Tab::Reports);
                true
            }
            KeyCode::Tab => {
                let next = self.tab.next();
                self.set_tab(next);
                true
            }
            KeyCode::BackTab => {
                let prev = self.tab.prev();
                self.set_tab(prev);
                true
            }
            KeyCode::Char('?') => {
                self.mode = InputMode::Help;
                true
            }
            KeyCode::Char('a') => {
                match self.tab {
                    Tab::Accounts => {
                        self.mode = InputMode::AddingAccount;
                        self.input_buffer.clear();
                        self.status_message = Some(
                            "输入: 名称,类型(wallet/bank_card/credit_card/invest),初始余额".into(),
                        );
                    }
                    Tab::Transactions => {
                        self.mode = InputMode::AddingTransaction;
                        self.input_buffer.clear();
                        self.status_message = Some(
                            "输入: account_id,category_id,amount,type(income/expense),description"
                                .into(),
                        );
                    }
                    Tab::Budget => {
                        self.mode = InputMode::AddingBudget;
                        self.input_buffer.clear();
                        self.status_message =
                            Some("输入: category_id,amount,period(monthly/weekly)".into());
                    }
                    _ => {}
                }
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.max_list_index();
                let next = (self.selected_index + 1).min(max.max(0) as usize);
                self.selected_index = next;
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
                true
            }
            KeyCode::Char('r') => {
                self.refresh_data();
                self.status_message = Some("🔄 数据已刷新".into());
                true
            }
            _ => true,
        }
    }

    fn submit_form(&mut self) {
        match self.mode {
            InputMode::AddingAccount => {
                let parts: Vec<&str> = self.input_buffer.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    let balance = parts
                        .get(2)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let svc = AccountService::new(&self.db);
                    match svc.create(parts[0], parts[1], "CNY", balance) {
                        Ok(id) => self.status_message = Some(format!("✅ 账户创建成功 ID={id}")),
                        Err(e) => self.status_message = Some(format!("❌ {e}")),
                    }
                }
            }
            InputMode::AddingTransaction => {
                let parts: Vec<&str> = self.input_buffer.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 5 {
                    let acc_id = parts[0].parse::<i64>().unwrap_or(0);
                    let cat_id = parts[1].parse::<i64>().unwrap_or(0);
                    let amount = parts[2].parse::<f64>().unwrap_or(0.0);
                    let svc = TxnService::new(&self.db);
                    match svc.create(acc_id, cat_id, amount, parts[3], parts[4], None) {
                        Ok(id) => self.status_message = Some(format!("✅ 交易创建成功 ID={id}")),
                        Err(e) => self.status_message = Some(format!("❌ {e}")),
                    }
                }
            }
            InputMode::AddingBudget => {
                let parts: Vec<&str> = self.input_buffer.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    let cat_id = parts[0].parse::<i64>().unwrap_or(0);
                    let amount = parts[1].parse::<f64>().unwrap_or(0.0);
                    let svc = BudgetService::new(&self.db);
                    match svc.set(cat_id, amount, parts[2]) {
                        Ok(id) => self.status_message = Some(format!("✅ 预算设置成功 ID={id}")),
                        Err(e) => self.status_message = Some(format!("❌ {e}")),
                    }
                }
            }
            _ => {}
        }
        self.mode = InputMode::Normal;
        self.input_buffer.clear();
        self.refresh_data();
    }

    /// 切换标签页并重置选择索引
    fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.selected_index = 0;
        self.refresh_data();
    }

    /// 当前标签页的列表最大索引（列表长度 - 1）
    fn max_list_index(&self) -> i32 {
        let len = match self.tab {
            Tab::Dashboard => self.recent_txns_summary.len().min(8),
            Tab::Accounts => {
                let svc = crate::engine::AccountService::new(&self.db);
                svc.list(false).map(|v| v.len()).unwrap_or(0)
            }
            Tab::Transactions => {
                let svc = crate::engine::TxnService::new(&self.db);
                svc.list(None, None, None, None, None, None, None, 20)
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
            Tab::Budget => {
                let svc = crate::engine::BudgetService::new(&self.db);
                svc.status().map(|v| v.len()).unwrap_or(0)
            }
            Tab::Reports => {
                let today = chrono::Local::now().date_naive();
                let month_start =
                    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
                let svc = crate::engine::ReportService::new(&self.db);
                svc.category_breakdown(crate::models::CategoryKind::Expense, month_start, today)
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
        };
        if len == 0 {
            -1 // 空列表
        } else {
            (len - 1) as i32
        }
    }

    pub fn on_tick(&mut self) {
        // 周期性地围栏 selected_index 到合法范围
        if self.selected_index as i32 > self.max_list_index() {
            self.selected_index = self.max_list_index().max(0) as usize;
        }
    }
}
