mod app;
mod event;
mod theme;
mod ui;
mod widgets;

use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::db::Database;
use crate::error::AppResult;

use app::App;
use event::{Event, EventHandler};
use ui::draw;

/// 启动 TUI 主循环
pub fn run_tui(db: Database) -> AppResult<()> {
    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建 App 状态和事件处理器
    let mut app = App::new(db);
    let mut event_handler = EventHandler::new(std::time::Duration::from_millis(200));

    // 主事件循环
    loop {
        // 绘制
        terminal.draw(|f| draw(f, &mut app))?;

        // 处理事件
        match event_handler.next()? {
            Event::Key(key) => {
                if !app.handle_key(key) {
                    break; // 退出
                }
            }
            Event::Tick => {
                app.on_tick();
            }
            Event::Resize(_, _) => {}
        }
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("👋 再见！");
    Ok(())
}
