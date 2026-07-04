use chrono::Datelike;
use crossterm::event::{KeyCode, KeyEvent};

use crate::db::Database;
use crate::engine::{AccountService, BudgetService, ReportService, TxnService};
use crate::models::cents_to_yuan;

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
            Tab::Dashboard => "概览",
            Tab::Accounts => "账户",
            Tab::Transactions => "交易",
            Tab::Budget => "预算",
            Tab::Reports => "报表",
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

/// 统计时间范围
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilter {
    All,
    Year(i32),
    Month(i32, u32),
    Day(chrono::NaiveDate),
}

impl DateFilter {
    pub fn label(&self) -> String {
        match self {
            DateFilter::All => "全部".into(),
            DateFilter::Year(y) => format!("{y}年"),
            DateFilter::Month(y, m) => format!("{y}-{m:02}月"),
            DateFilter::Day(d) => d.to_string(),
        }
    }
    /// 统计起始日期
    pub fn start_date(&self) -> Option<chrono::NaiveDate> {
        match self {
            DateFilter::All => None,
            DateFilter::Year(y) => chrono::NaiveDate::from_ymd_opt(*y, 1, 1),
            DateFilter::Month(y, m) => chrono::NaiveDate::from_ymd_opt(*y, *m, 1),
            DateFilter::Day(d) => Some(*d),
        }
    }
    /// 统计结束日期
    pub fn end_date(&self) -> Option<chrono::NaiveDate> {
        match self {
            DateFilter::All => None,
            DateFilter::Year(y) => chrono::NaiveDate::from_ymd_opt(*y, 12, 31),
            DateFilter::Month(y, m) => {
                let last = chrono::NaiveDate::from_ymd_opt(*y, *m + 1, 1)
                    .unwrap_or(chrono::NaiveDate::from_ymd_opt(*y + 1, 1, 1).unwrap())
                    .pred_opt()
                    .unwrap();
                Some(last)
            }
            DateFilter::Day(d) => Some(*d),
        }
    }
    /// 从用户输入解析: "2026" / "2026-07" / "2026-07-01" / ""=全部
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.is_empty() { return Some(DateFilter::All); }
        if s.len() == 4 {
            let y: i32 = s.parse().ok()?;
            return Some(DateFilter::Year(y));
        }
        if s.len() == 7 && s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 2 {
                let y: i32 = parts[0].parse().ok()?;
                let m: u32 = parts[1].parse().ok()?;
                if m >= 1 && m <= 12 { return Some(DateFilter::Month(y, m)); }
            }
        }
        if s.len() == 10 && s.contains('-') {
            let d = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
            return Some(DateFilter::Day(d));
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingTransaction,
    AddingAccount,
    AddingBudget,
    Help,
    DeleteConfirm,
    AccountPicker,
    TimePicker,
}

pub struct App {
    pub db: Database,
    pub tab: Tab,
    pub mode: InputMode,
    pub should_quit: bool,

    pub input_buffer: String,
    pub input_fields: Vec<String>,
    #[allow(dead_code)]
    pub current_field: usize,
    pub status_message: Option<String>,

    pub selected_index: usize,

    /// 统计时间范围
    pub date_filter: DateFilter,
    /// None = 所有账户，Some(id) = 仅此账户
    pub active_account_filter: Option<i64>,
    /// 缓存当前筛选账户名称
    pub active_account_name: Option<String>,

    // 数据缓存
    pub account_count: usize,
    pub total_balance_cents: i64,
    pub month_income: i64,
    pub month_expense: i64,
    pub recent_txns_summary: Vec<String>,
    pub budget_alerts: Vec<String>,

    // 交易缓存（预格式化行，支持滚动翻页）
    pub txn_lines: Vec<String>,
    pub txn_total: usize,          // 总交易数
    pub visible_offset: usize,     // 可视窗口起始偏移

    // 总预算（特殊，单独管理）
    pub total_budget_cents: i64,
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
            date_filter: DateFilter::All,
            active_account_filter: None,
            active_account_name: None,
            account_count: 0,
            total_balance_cents: 0,
            month_income: 0,
            month_expense: 0,
            recent_txns_summary: Vec::new(),
            budget_alerts: Vec::new(),
            txn_lines: Vec::new(),
            txn_total: 0,
            visible_offset: 0,
            total_budget_cents: 0,
        };
        app.refresh_data();
        app
    }

    /// 刷新所有数据（尊重账户筛选）
    pub fn refresh_data(&mut self) {
        let acc_svc = AccountService::new(&self.db);

        // 更新筛选账户名
        if let Some(id) = self.active_account_filter {
            if let Ok(acc) = acc_svc.get(id) {
                self.active_account_name = Some(acc.name);
            }
        }

        // 余额：筛选时只算该账户，否则算所有
        if let Ok(accounts) = acc_svc.list(false) {
            self.account_count = accounts.len();
            if let Some(fid) = self.active_account_filter {
                self.total_balance_cents = accounts
                    .iter()
                    .filter(|a| a.id == Some(fid))
                    .map(|a| a.balance_cents)
                    .sum();
            } else {
                self.total_balance_cents = accounts.iter().map(|a| a.balance_cents).sum();
            }
        }

        let today = chrono::Local::now().date_naive();
        let from = self.date_filter.start_date();

        // 仪表盘收支统计
        let (qs, qv): (&str, Option<String>) = match from {
            Some(d) => (" AND txn_date>=?2", Some(d.to_string())),
            None => ("", None),
        };
        let income_sql = format!("SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE type='income'{qs}");
        let expense_sql = format!("SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE type='expense'{qs}");
        self.month_income = self.db.conn().query_row(
            &income_sql, rusqlite::params![qv.clone()], |row| row.get::<_,i64>(0)).unwrap_or(0);
        self.month_expense = self.db.conn().query_row(
            &expense_sql, rusqlite::params![qv], |row| row.get::<_,i64>(0)).unwrap_or(0);

        // Dashboard最近交易
        let txn_svc = TxnService::new(&self.db);
        if let Ok(txns) =
            txn_svc.list(self.active_account_filter, None, None, from, None, None, None, 20)
        {
            self.recent_txns_summary = txns.iter().map(|t| {
                let sign = match t.txn_type {
                    crate::models::TxnType::Income => "+",
                    crate::models::TxnType::Expense => "-",
                    crate::models::TxnType::Transfer => ">>",
                };
                format!("{}  {}{:<12} {}", t.txn_date, sign, cents_to_yuan(t.amount_cents), t.description)
            }).collect();
        }

        // 交易页缓存（大容量，支持滚动）
        if let Ok(all_txns) =
            txn_svc.list(self.active_account_filter, None, None, from, None, None, None, 1000)
        {
            self.txn_total = all_txns.len();
            self.txn_lines = all_txns.iter().map(|t| {
                let sign = match t.txn_type {
                    crate::models::TxnType::Income => "+",
                    crate::models::TxnType::Expense => "-",
                    crate::models::TxnType::Transfer => ">>",
                };
                format!(
                    "{:<6} {:<12} {:<8} {:<8} {}{:<11} {}",
                    t.id.unwrap_or(0),
                    t.txn_date,
                    t.txn_type.display_name(),
                    t.category_id.map_or("-".into(), |c| c.to_string()),
                    sign,
                    cents_to_yuan(t.amount_cents),
                    t.description,
                )
            }).collect();
            self.visible_offset = self.visible_offset.min(self.txn_lines.len().saturating_sub(1));
        }

        // 预算告警
        let budget_svc = BudgetService::new(&self.db);
        if let Ok(alerts) = budget_svc.status(from) {
            self.budget_alerts = alerts
                .iter()
                .filter(|s| s.progress_pct >= 80.0)
                .map(|s| format!("{} {}: {:.0}%", s.category_icon, s.category_name, s.progress_pct))
                .collect();
        }
    }

    /// 选中指定账户作为筛选
    fn select_account(&mut self, account_id: Option<i64>) {
        self.active_account_filter = account_id;
        self.active_account_name = None; // refresh_data 会更新
        self.selected_index = 0;
        self.refresh_data();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // 时间选择器模式
        if self.mode == InputMode::TimePicker {
            match key.code {
                KeyCode::Enter => {
                    let filter = DateFilter::parse(&self.input_buffer)
                        .unwrap_or(self.date_filter);
                    self.date_filter = filter;
                    self.mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.refresh_data();
                    self.status_message = Some(format!("时间范围: {}", self.date_filter.label()));
                    return true;
                }
                KeyCode::Esc => {
                    self.mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.status_message = Some("已取消".into());
                    return true;
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    return true;
                }
                KeyCode::Char(ch) => {
                    self.input_buffer.push(ch);
                    return true;
                }
                _ => return true,
            }
        }

        // 账户选择器模式
        if self.mode == InputMode::AccountPicker {
            self.mode = InputMode::Normal;
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let idx = c.to_digit(10).unwrap() as usize;
                    let accounts = AccountService::new(&self.db).list(false).unwrap_or_default();
                    if idx == 0 {
                        self.select_account(None);
                        self.status_message = Some("显示全部账户".into());
                    } else if idx <= accounts.len() {
                        self.select_account(accounts[idx - 1].id);
                        self.status_message =
                            Some(format!("已切换到: {}", accounts[idx - 1].name));
                    }
                    return true;
                }
                KeyCode::Esc => {
                    self.status_message = Some("已取消".into());
                    return true;
                }
                _ => {
                    self.status_message = Some("已取消账户选择".into());
                    return true;
                }
            }
        }

        // 删除确认
        if self.mode == InputMode::DeleteConfirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.do_delete();
                    self.mode = InputMode::Normal;
                    return true;
                }
                _ => {
                    self.mode = InputMode::Normal;
                    self.status_message = Some("已取消删除".into());
                    return true;
                }
            }
        }

        if key.code == KeyCode::Esc {
            if self.mode != InputMode::Normal {
                self.mode = InputMode::Normal;
                self.input_buffer.clear();
                self.status_message = None;
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
            KeyCode::Char('1') => { self.set_tab(Tab::Dashboard); true }
            KeyCode::Char('2') => { self.set_tab(Tab::Accounts); true }
            KeyCode::Char('3') => { self.set_tab(Tab::Transactions); true }
            KeyCode::Char('4') => { self.set_tab(Tab::Budget); true }
            KeyCode::Char('5') => { self.set_tab(Tab::Reports); true }
            KeyCode::Tab => { let n = self.tab.next(); self.set_tab(n); true }
            KeyCode::BackTab => { let p = self.tab.prev(); self.set_tab(p); true }
            KeyCode::Char('?') | KeyCode::Char('h') => { self.mode = InputMode::Help; true }

            KeyCode::Char('a') => {
                match self.tab {
                    Tab::Accounts => {
                        self.mode = InputMode::AddingAccount;
                        self.input_buffer.clear();
                        self.status_message =
                            Some("输入: 名称,类型(wallet/bank_card/credit_card/invest),初始余额".into());
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
                            Some("输入: 商户名,金额  或  总额,金额".into());
                    }
                    _ => {
                        self.status_message =
                            Some("当前页面不支持添加，切换到账户/交易/预算页再试".into());
                    }
                }
                true
            }

            KeyCode::Enter => {
                // 在账户页按 Enter = 选中该账户作为筛选
                if self.tab == Tab::Accounts {
                    let accounts = AccountService::new(&self.db).list(false).unwrap_or_default();
                    if let Some(acc) = accounts.get(self.selected_index) {
                        if self.active_account_filter == acc.id {
                            // 再次按 Enter 取消筛选
                            self.select_account(None);
                            self.status_message = Some("显示全部账户".into());
                        } else {
                            self.select_account(acc.id);
                            self.status_message =
                                Some(format!("已选中账户: {}", acc.name));
                        }
                    }
                } else {
                    self.status_message = Some("使用 a 添加, d 删除, f 选账户, r 刷新, q 退出".into());
                }
                true
            }

            KeyCode::Char('f') => {
                // 打开账户选择器，显示数字提示
                let accounts = AccountService::new(&self.db).list(false).unwrap_or_default();
                if accounts.is_empty() {
                    self.status_message = Some("暂无账户".into());
                    return true;
                }
                self.mode = InputMode::AccountPicker;
                let mut hint = String::from("选择账户: 按 0=全部");
                for (i, a) in accounts.iter().enumerate().take(9) {
                    hint.push_str(&format!(", {}=[{}]", i + 1, a.name));
                }
                self.status_message = Some(hint);
                true
            }

            KeyCode::Char('0') => {
                self.select_account(None);
                self.status_message = Some("显示全部账户".into());
                true
            }

            KeyCode::Char('d') => {
                if self.selected_index < self.max_list_index().max(0) as usize + 1 {
                    self.mode = InputMode::DeleteConfirm;
                    self.status_message = Some("确认删除? 按 y 确认, 其他键取消".into());
                }
                true
            }

            KeyCode::Char('j') | KeyCode::Down => {
                let max = self.max_list_index();
                let next = self.selected_index + 1;
                self.selected_index = next.min(max.max(0) as usize);
                // 交易页自动滚动偏移
                if self.tab == Tab::Transactions {
                    let vis = visible_rows();
                    if self.selected_index >= self.visible_offset + vis {
                        self.visible_offset = (self.selected_index + 1).saturating_sub(vis);
                    }
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
                // 交易页自动滚动偏移
                if self.tab == Tab::Transactions {
                    if self.selected_index < self.visible_offset {
                        self.visible_offset = self.selected_index;
                    }
                }
                true
            }
            KeyCode::Char('t') => {
                self.mode = InputMode::TimePicker;
                self.input_buffer.clear();
                self.status_message = Some("输入日期: YYYY / YYYY-MM / YYYY-MM-DD / 空=全部, Enter确认".into());
                true
            }
            KeyCode::Char('r') => {
                self.refresh_data();
                self.status_message = Some("数据已刷新".into());
                true
            }
            _ => true,
        }
    }

    fn do_delete(&mut self) {
        match self.tab {
            Tab::Transactions => {
                let svc = TxnService::new(&self.db);
                if let Ok(txns) = svc.list(
                    self.active_account_filter, None, None, None, None, None, None, 20,
                ) {
                    if let Some(txn) = txns.get(self.selected_index) {
                        if let Some(id) = txn.id {
                            match svc.delete(id) {
                                Ok(()) => self.status_message = Some(format!("已删除交易 ID={id}")),
                                Err(e) => self.status_message = Some(format!("删除失败: {e}")),
                            }
                        }
                    }
                }
            }
            Tab::Accounts => {
                let svc = AccountService::new(&self.db);
                if let Ok(accounts) = svc.list(false) {
                    if let Some(acc) = accounts.get(self.selected_index) {
                        if let Some(id) = acc.id {
                            match svc.delete(id, true) {
                                Ok(()) => self.status_message = Some(format!("已删除账户 ID={id}")),
                                Err(e) => self.status_message = Some(format!("删除失败: {e}")),
                            }
                        }
                    }
                }
            }
            Tab::Budget => {
                let svc = BudgetService::new(&self.db);
                let from = self.date_filter.start_date();
                if let Ok(statuses) = svc.status(from) {
                    if let Some(s) = statuses.get(self.selected_index) {
                        match svc.delete(s.budget_id) {
                            Ok(()) => self.status_message =
                                Some(format!("已删除预算: {}", s.category_name)),
                            Err(e) => self.status_message = Some(format!("删除失败: {e}")),
                        }
                    }
                }
            }
            _ => {
                self.status_message = Some("当前页面不支持删除".into());
            }
        }
        self.refresh_data();
    }

    fn submit_form(&mut self) {
        match self.mode {
            InputMode::AddingAccount => {
                let parts: Vec<&str> = self.input_buffer.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    let balance = parts.get(2).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let svc = AccountService::new(&self.db);
                    match svc.create(parts[0], parts[1], "CNY", balance) {
                        Ok(id) => self.status_message = Some(format!("账户创建成功 ID={id}")),
                        Err(e) => self.status_message = Some(format!("{e}")),
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
                        Ok(id) => self.status_message = Some(format!("交易创建成功 ID={id}")),
                        Err(e) => self.status_message = Some(format!("{e}")),
                    }
                }
            }
            InputMode::AddingBudget => {
                let parts: Vec<&str> = self.input_buffer.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let amount = parts[1].parse::<f64>().unwrap_or(0.0);
                    if name == "总额" || name == "总预算" {
                        self.total_budget_cents = (amount * 100.0) as i64;
                        self.status_message = Some(format!("总预算设为 ¥{amount}"));
                    } else {
                        // 按名查找分类ID
                        let conn = self.db.conn();
                        let cat_id: Option<i64> = conn.query_row(
                            "SELECT id FROM categories WHERE name=?1",
                            rusqlite::params![name],
                            |row| row.get(0),
                        ).ok();
                        match cat_id {
                            Some(id) => {
                                let svc = BudgetService::new(&self.db);
                                let from = self.date_filter.start_date();
                                match svc.set(id, amount, "monthly", from) {
                                    Ok(_) => self.status_message =
                                        Some(format!("{name} 预算 ¥{amount}")),
                                    Err(e) => self.status_message = Some(format!("{e}")),
                                }
                            }
                            None => {
                                self.status_message =
                                    Some(format!("未找到商户: {name}"));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.mode = InputMode::Normal;
        self.input_buffer.clear();
        self.refresh_data();
    }

    fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.selected_index = 0;
        self.visible_offset = 0;
        self.refresh_data();
    }

    fn max_list_index(&self) -> i32 {
        let len = match self.tab {
            Tab::Dashboard => self.recent_txns_summary.len().min(15),
            Tab::Accounts => {
                AccountService::new(&self.db)
                    .list(false)
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
            Tab::Transactions => self.txn_lines.len().saturating_sub(1),
            Tab::Budget => {
                BudgetService::new(&self.db)
                    .status(self.date_filter.start_date())
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
            Tab::Reports => {
                let today = chrono::Local::now().date_naive();
                let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
                ReportService::new(&self.db)
                    .category_breakdown(crate::models::CategoryKind::Expense, year_start, today)
                    .map(|v| v.len())
                    .unwrap_or(0)
            }
        };
        if len == 0 { -1 } else { (len - 1) as i32 }
    }

    pub fn on_tick(&mut self) {
        if self.selected_index as i32 > self.max_list_index() {
            self.selected_index = self.max_list_index().max(0) as usize;
        }
    }
}

/// 估算交易列表可见行数（约30行用于渲染）
pub fn visible_rows() -> usize { 30 }
