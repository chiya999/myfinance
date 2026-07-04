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

    // 帮助覆盖层
    if app.mode == InputMode::Help {
        draw_help_overlay(f, area, &theme);
    }

    // 删除确认覆盖层
    if app.mode == InputMode::DeleteConfirm {
        draw_delete_confirm(f, area, &theme, app.status_message.as_deref().unwrap_or(""));
    }

    // 状态栏
    draw_status_bar(f, chunks[3], app, &theme);
}

fn draw_title(f: &mut Frame, area: Rect, theme: &Theme) {
    let title = Paragraph::new("myfinance")
        .style(Style::default().fg(theme.primary).bold())
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

    // 右侧：时间范围 + 账户标识 + 快捷键
    let acc_label = app.active_account_name.as_deref().unwrap_or("全部账户");
    let acc_color = if app.active_account_filter.is_some() { theme.warning } else { theme.muted };
    let tr_label = app.date_filter.label();
    let hint = Paragraph::new(format!(
        "[{}] [{}] t换范围 f选账户 ?帮助 q退出",
        tr_label,
        acc_label,
    ))
    .style(Style::default().fg(acc_color))
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
    for (i, txn) in app.recent_txns_summary.iter().enumerate().take(15) {
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
    let vis = super::app::visible_rows();
    let start = app.visible_offset;
    let end = (start + vis).min(app.txn_lines.len());

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!(
            "📝 交易记录 ({}/{}) ← j/k滚动  账户: {}  范围: {}",
            app.txn_total,
            app.txn_lines.len(),
            app.active_account_name.as_deref().unwrap_or("全部"),
            app.date_filter.label(),
        ),
        Style::default().fg(theme.primary).bold(),
    ))];

    if app.txn_lines.is_empty() {
        lines.push(Line::from(Span::styled("  (暂无交易)", Style::default().fg(theme.muted))));
    } else {
        let header = format!("{:<6} {:<12} {:<8} {:<8} {:<12} {}", "ID", "日期", "类型", "分类", "金额", "描述");
        lines.push(Line::from(Span::styled(header, Style::default().fg(theme.muted))));
        lines.push(Line::from("─".repeat(72)));

        for i in start..end {
            let line = &app.txn_lines[i];
            let style = if i == app.selected_index {
                Style::default().fg(Color::White).bg(theme.highlight)
            } else {
                Style::default().fg(theme.text)
            };
            lines.push(Line::from(Span::styled(line.as_str(), style)));
        }
    }

    let p = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);
}

fn draw_budget_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let from = app.date_filter.start_date();
    let today = chrono::Local::now().date_naive();

    // 按时间范围计算总支出
    let total_spent: i64 = if let Some(d) = from {
        app.db.conn().query_row(
            "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE type='expense' AND txn_date>=?1",
            rusqlite::params![d.to_string()],
            |row| row.get(0),
        ).unwrap_or(0)
    } else {
        app.db.conn().query_row(
            "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE type='expense'",
            [], |row| row.get(0),
        ).unwrap_or(0)
    };

    let svc = crate::engine::BudgetService::new(&app.db);
    let statuses = svc.status(from).unwrap_or_default();

    // 布局：标题 + 总额行 + 各商户
    let count = statuses.len().min(20) + 2; // title + total
    let mut constraints: Vec<Constraint> = vec![Constraint::Length(2)];
    for _ in 0..count { constraints.push(Constraint::Length(1)); }
    constraints.push(Constraint::Min(0));
    let rows = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    // 标题
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("🎯 预算执行", Style::default().fg(theme.primary).bold()))),
        rows[0],
    );

    // 总额行
    let total_pct = if app.total_budget_cents > 0 {
        total_spent as f64 / app.total_budget_cents as f64 * 100.0
    } else { 0.0 };
    let total_color = if total_pct >= 100.0 { theme.danger } else if total_pct >= 80.0 { theme.warning } else { theme.success };
    let total_splits = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)]).split(rows[1]);
    f.render_widget(
        Paragraph::new(format!("总额预算: ¥{}", cents_to_yuan(app.total_budget_cents))).style(Style::default().fg(theme.warning)),
        total_splits[0],
    );
    let bar = progress_bar_unicode(total_pct, 30, total_color);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(bar, Style::default().fg(total_color)),
            Span::raw(format!(" {:>5.1}% ¥{}/¥{}", total_pct, cents_to_yuan(total_spent), cents_to_yuan(app.total_budget_cents))),
        ])),
        total_splits[1],
    );

    // 各商户预算
    if statuses.is_empty() {
        f.render_widget(
            Paragraph::new("  暂无预算。按 a 添加: 商户名,金额  如: 美团,3000").style(Style::default().fg(theme.muted)),
            rows[2],
        );
        return;
    }

    for (i, s) in statuses.iter().enumerate().take(20) {
        let row_idx = i + 2;
        if row_idx >= rows.len() { break; }
        let splits = Layout::default().direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)]).split(rows[row_idx]);

        let color = if s.progress_pct >= 100.0 { theme.danger }
                    else if s.progress_pct >= 80.0 { theme.warning }
                    else { theme.success };
        let name = if s.category_name.chars().count() > 14 {
            format!("{}..", s.category_name.chars().take(13).collect::<String>())
        } else { s.category_name.clone() };

        f.render_widget(
            Paragraph::new(format!("{}: ¥{}", name, cents_to_yuan(s.amount_cents))).style(Style::default().fg(theme.text)),
            splits[0],
        );
        let bar = progress_bar_unicode(s.progress_pct, 30, color);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(bar, Style::default().fg(color)),
                Span::raw(format!(" {:>5.1}% ¥{}/¥{}", s.progress_pct, cents_to_yuan(s.spent_cents), cents_to_yuan(s.amount_cents))),
            ])),
            splits[1],
        );
    }
}

fn draw_reports_tab(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let today = chrono::Local::now().date_naive();
    let from = app.date_filter.start_date().unwrap_or(
        chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let rpt_svc = crate::engine::ReportService::new(&app.db);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // 摘要
    if let Ok(summary) = rpt_svc.summary(from, today) {
        let range_label = app.date_filter.label();
        let text = format!(
            "{} 收支总览 ({}→今)\n  收入: ¥{}  支出: ¥{}  净额: ¥{}  交易: {}笔",
            if app.active_account_filter.is_some() { "📊" } else { "📊" },
            range_label,
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
    let expense_total: i64 = app.db.conn().query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE type='expense'",
        [], |row| row.get(0)).unwrap_or(0);

    match rpt_svc.category_breakdown(crate::models::CategoryKind::Expense, from, today) {
        Ok(breakdown) if !breakdown.is_empty() => {
            // 用 Layout 强制对齐：左列=名字，右列=进度条+数字
            let mut rows: Vec<ratatui::layout::Rect> = Vec::new();
            let mut constraints: Vec<Constraint> = vec![Constraint::Length(2)]; // 标题行
            for _ in breakdown.iter().take(10) {
                constraints.push(Constraint::Length(1));
            }
            let row_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(chunks[1]);

            // 标题
            let title = Paragraph::new(Line::from(Span::styled(
                format!("📋 支出分类 (合计: ¥{})", cents_to_yuan(expense_total)),
                Style::default().fg(theme.primary).bold(),
            )));
            f.render_widget(title, row_chunks[0]);

            for (i, b) in breakdown.iter().take(10).enumerate() {
                let splits = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .split(row_chunks[i + 1]);

                let name = &b.category_name;
                let short: String = if name.chars().count() > 16 {
                    name.chars().take(15).collect::<String>() + ".."
                } else {
                    name.to_string()
                };
                let label = Paragraph::new(short).style(Style::default().fg(theme.text));
                f.render_widget(label, splits[0]);

                let bar = progress_bar_unicode(b.percentage, 30, theme.danger);
                let right = Paragraph::new(Line::from(vec![
                    Span::styled(bar, Style::default().fg(theme.danger)),
                    Span::raw(format!(" {:>5.1}% ¥{}", b.percentage, cents_to_yuan(b.amount_cents))),
                ]));
                f.render_widget(right, splits[1]);
            }

        }
        Ok(_) => {
            let p = Paragraph::new(format!(
                "📋 支出分类\n  数据库中支出合计 ¥{}，但分类聚合并未返回结果。\n  请确认导入的交易category_id是否为有效支出分类(1-11)。",
                cents_to_yuan(expense_total)
            )).style(Style::default().fg(theme.warning));
            f.render_widget(p, chunks[1]);
        }
        Err(e) => {
            let p = Paragraph::new(format!("📋 支出分类\n  查询错误: {e}"))
                .style(Style::default().fg(theme.danger));
            f.render_widget(p, chunks[1]);
        }
    }
}

/// 帮助覆盖层
fn draw_help_overlay(f: &mut Frame, area: Rect, theme: &Theme) {
    let popup_area = centered_rect(60, 70, area);
    f.render_widget(
        ratatui::widgets::Clear,
        popup_area,
    );
    let lines = vec![
        Line::from(Span::styled(" 快捷键帮助", Style::default().fg(theme.primary).bold())),
        Line::from(""),
        Line::from(" 1-5     切换标签页"),
        Line::from(" Tab     下一标签"),
        Line::from(" j/k/↑/↓  上下导航"),
        Line::from(" a       添加 (账户/交易/预算)"),
        Line::from(" Enter   查看详情 / 确认"),
        Line::from(" d       删除当前选中项"),
        Line::from(" f       切换账户筛选"),
        Line::from(" r       刷新数据"),
        Line::from(" h/?     显示此帮助"),
        Line::from(" q       退出程序"),
        Line::from(" Esc     取消 / 退出"),
        Line::from(""),
        Line::from(Span::styled(" 按任意键关闭", Style::default().fg(theme.muted))),
    ];
    let p = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" ? 帮助 ")
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().bg(Color::Rgb(10, 12, 18))),
        )
        .style(Style::default().fg(theme.text));
    f.render_widget(p, popup_area);
}

/// 删除确认覆盖层
fn draw_delete_confirm(f: &mut Frame, area: Rect, theme: &Theme, msg: &str) {
    let popup_area = centered_rect(40, 15, area);
    f.render_widget(ratatui::widgets::Clear, popup_area);
    let lines = vec![
        Line::from(Span::styled(msg, Style::default().fg(theme.warning).bold())),
    ];
    let p = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" 确认删除 ")
                .border_style(Style::default().fg(theme.danger))
                .style(Style::default().bg(Color::Rgb(10, 12, 18))),
        )
        .style(Style::default().fg(theme.text));
    f.render_widget(p, popup_area);
}

/// 居中矩形
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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

fn progress_bar_unicode(pct: f64, width: usize, _color: Color) -> String {
    let filled = ((pct / 100.0 * width as f64).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}
