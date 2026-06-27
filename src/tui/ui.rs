use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};

use super::app::{App, InputMode, Tab};
use super::theme::Theme;
use crate::models::cents_to_yuan;

/// 主绘制函数
pub fn draw(f: &mut Frame, app: &mut App) {
    let theme = Theme::default();
    let area = f.area();

    // 垂直布局: 标题栏 + 标签页 + 内容 + 状态栏
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 标题
            Constraint::Length(3), // 标签页
            Constraint::Min(3),    // 内容
            Constraint::Length(1), // 状态栏
        ])
        .split(area);

    // 标题
    draw_title(f, chunks[0], &theme);

    // 标签页
    draw_tabs(f, chunks[1], app, &theme);

    // 内容区
    match app.tab {
        Tab::Dashboard => draw_dashboard(f, chunks[2], app, &theme),
        Tab::Accounts => draw_accounts_tab(f, chunks[2], app, &theme),
        Tab::Transactions => draw_transactions_tab(f, chunks[2], app, &theme),
        Tab::Budget => draw_budget_tab(f, chunks[2], app, &theme),
        Tab::Reports => draw_reports_tab(f, chunks[2], app, &theme),
    }

    // 状态栏
    draw_status_bar(f, chunks[3], app, &theme);
}

fn draw_title(f: &mut Frame, area: Rect, theme: &Theme) {
    let title = Paragraph::new("💰 myfinance")
        .style(Style::default().fg(theme.primary))
        .alignment(Alignment::Left);
    f.render_widget(title, area);
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let text = t.name();
            if *t == app.tab {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(theme.primary).bold(),
                ))
            } else {
                Line::from(Span::styled(text, Style::default().fg(theme.muted)))
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border)),
        )
        .select(app.tab as usize)
        .style(Style::default().fg(theme.text));

    // 右侧快捷键提示
    let hint = Paragraph::new("1-5:切换 Tab q:退出 a:添加 r:刷新 ?:帮助")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Right);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    f.render_widget(tabs, chunks[0]);
    f.render_widget(hint, chunks[1]);
}

fn draw_dashboard(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // 概览卡片
            Constraint::Length(3), // 预算告警
            Constraint::Min(0),    // 最近交易
        ])
        .split(area);

    // 概览卡片
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[0]);

    let overview_items = [
        (
            "本月收入",
            format!("¥{}", cents_to_yuan(app.month_income)),
            theme.success,
        ),
        (
            "本月支出",
            format!("¥{}", cents_to_yuan(app.month_expense)),
            theme.danger,
        ),
        (
            "净资产",
            format!("¥{}", cents_to_yuan(app.total_balance_cents)),
            theme.primary,
        ),
    ];

    for (i, (label, value, color)) in overview_items.iter().enumerate() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));
        let text = Text::from(vec![
            Line::from(Span::styled(*label, Style::default().fg(theme.muted))),
            Line::from(Span::styled(
                value.as_str(),
                Style::default().fg(*color).bold(),
            )),
        ]);
        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, cards[i]);
    }

    // 预算告警
    if !app.budget_alerts.is_empty() {
        let alert_text = app.budget_alerts.join("  │  ");
        let p = Paragraph::new(alert_text)
            .style(Style::default().fg(theme.warning))
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_style(Style::default().fg(theme.warning)),
            );
        f.render_widget(p, chunks[1]);
    }

    // 最近交易
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        "📋 最近交易",
        Style::default().fg(theme.primary).bold(),
    ))];
    for (i, txn) in app.recent_txns_summary.iter().enumerate().take(8) {
        let style = if i == app.selected_index {
            Style::default().fg(Color::White).bg(theme.highlight)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::from(Span::styled(txn.as_str(), style)));
    }
    if app.recent_txns_summary.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (暂无交易记录)",
            Style::default().fg(theme.muted),
        )));
    }
    let p = Paragraph::new(Text::from(lines));
    f.render_widget(p, chunks[2]);
}

fn draw_accounts_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let acc_svc = crate::engine::AccountService::new(&app.db);
    let accounts = acc_svc.list(false).unwrap_or_default();

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("{:<6} {:<16} {:<10} {:<10}", "ID", "名称", "类型", "余额"),
        Style::default().fg(theme.primary).bold(),
    ))];
    lines.push(Line::from("─".repeat(50)));

    for (i, a) in accounts.iter().enumerate() {
        let style = if i == app.selected_index {
            Style::default().fg(Color::White).bg(theme.highlight)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::from(Span::styled(
            format!(
                "{:<6} {:<16} {:<10} ¥{}",
                a.id.unwrap(),
                a.name,
                a.kind.display_name(),
                cents_to_yuan(a.balance_cents)
            ),
            style,
        )));
    }

    let p = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("💳 账户列表")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);
}

fn draw_transactions_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let txn_svc = crate::engine::TxnService::new(&app.db);
    let txns = txn_svc
        .list(None, None, None, None, None, None, None, 20)
        .unwrap_or_default();

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!(
            "{:<6} {:<12} {:<8} {:<8} {:<12} {}",
            "ID", "日期", "类型", "分类", "金额", "描述"
        ),
        Style::default().fg(theme.primary).bold(),
    ))];
    lines.push(Line::from("─".repeat(70)));

    for (i, t) in txns.iter().enumerate() {
        let style = if i == app.selected_index {
            Style::default().fg(Color::White).bg(theme.highlight)
        } else {
            Style::default().fg(theme.text)
        };
        let sign = match t.txn_type {
            crate::models::TxnType::Income => "+",
            crate::models::TxnType::Expense => "-",
            crate::models::TxnType::Transfer => "↔",
        };
        lines.push(Line::from(Span::styled(
            format!(
                "{:<6} {:<12} {:<8} {:<8} {}{:<11} {}",
                t.id.unwrap(),
                t.txn_date,
                t.txn_type.display_name(),
                t.category_id.map_or("-".to_string(), |c| c.to_string()),
                sign,
                cents_to_yuan(t.amount_cents),
                t.description,
            ),
            style,
        )));
    }

    let p = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("📝 交易记录")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);
}

fn draw_budget_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let svc = crate::engine::BudgetService::new(&app.db);
    let statuses = svc.status().unwrap_or_default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // 总览
    let total_budget: i64 = statuses.iter().map(|s| s.amount_cents).sum();
    let total_spent: i64 = statuses.iter().map(|s| s.spent_cents).sum();
    let overview = Paragraph::new(format!(
        "总预算: ¥{}  已花: ¥{}  剩余: ¥{}",
        cents_to_yuan(total_budget),
        cents_to_yuan(total_spent),
        cents_to_yuan(total_budget - total_spent),
    ))
    .style(Style::default().fg(theme.text));
    f.render_widget(overview, chunks[0]);

    // 详细进度条
    let mut lines: Vec<Line> = Vec::new();
    for s in &statuses {
        let color = if s.progress_pct >= 100.0 {
            theme.danger
        } else if s.progress_pct >= 80.0 {
            theme.warning
        } else {
            theme.success
        };
        let bar = progress_bar_unicode(s.progress_pct, 30, color);
        lines.push(Line::from(vec![
            Span::raw(format!("{}{:<8} ", s.category_icon, s.category_name)),
            Span::styled(bar, Style::default().fg(color)),
            Span::raw(format!(
                " {:>5.1}% ¥{}/¥{}",
                s.progress_pct,
                cents_to_yuan(s.spent_cents),
                cents_to_yuan(s.amount_cents)
            )),
        ]));
    }

    let p = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title("🎯 预算执行")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, chunks[1]);
}

fn draw_reports_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let today = chrono::Local::now().date_naive();
    let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let rpt_svc = crate::engine::ReportService::new(&app.db);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // 摘要
    if let Ok(summary) = rpt_svc.summary(month_start, today) {
        let text = format!(
            "📊 本月 ({}) 收支总览\n  收入: ¥{}  支出: ¥{}  净额: ¥{}  交易: {}笔",
            today.format("%Y-%m"),
            cents_to_yuan(summary.total_income),
            cents_to_yuan(summary.total_expense),
            cents_to_yuan(summary.net),
            summary.transaction_count,
        );
        let p = Paragraph::new(text)
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
        f.render_widget(p, chunks[0]);
    }

    // 分类明细
    if let Ok(breakdown) =
        rpt_svc.category_breakdown(crate::models::CategoryKind::Expense, month_start, today)
    {
        let mut lines: Vec<Line> = vec![Line::from(Span::styled(
            "📋 支出分类",
            Style::default().fg(theme.primary).bold(),
        ))];
        for b in breakdown.iter().take(10) {
            let bar = progress_bar_unicode(b.percentage, 20, theme.danger);
            lines.push(Line::from(vec![
                Span::raw(format!("{} {:<10} ", b.category_icon, b.category_name)),
                Span::styled(bar, Style::default().fg(theme.danger)),
                Span::raw(format!(
                    " {:.1}% ¥{}",
                    b.percentage,
                    cents_to_yuan(b.amount_cents)
                )),
            ]));
        }
        let p = Paragraph::new(Text::from(lines));
        f.render_widget(p, chunks[1]);
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let left = match &app.mode {
        InputMode::Normal => format!(" {} | j/k:导航 ENTER:确认", app.tab.name()),
        InputMode::Help => " ?:帮助 Esc:返回".into(),
        _ => format!(" 📝 输入: {}▊ (Enter:确认 Esc:取消)", app.input_buffer),
    };

    let right = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else if app.mode == InputMode::Help {
        "快捷键: 1-5:标签 q:退出 a:添加 r:刷新 j/k:上下 Tab:切页".into()
    } else {
        String::new()
    };

    let status = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    f.render_widget(
        Paragraph::new(left).style(Style::default().fg(theme.text).bg(theme.surface)),
        status[0],
    );
    f.render_widget(
        Paragraph::new(right)
            .style(Style::default().fg(theme.muted).bg(theme.surface))
            .alignment(Alignment::Right),
        status[1],
    );
}

fn progress_bar_unicode(pct: f64, width: usize, color: Color) -> String {
    let filled = ((pct / 100.0 * width as f64).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}
