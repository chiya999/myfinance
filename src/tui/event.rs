use std::time::Duration;

use crate::error::AppResult;
use crossterm::event::{self, KeyEvent, KeyEventKind};

/// 事件类型
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    #[allow(dead_code)]
    Resize(u16, u16),
}

/// 事件处理器
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// 获取下一个有效事件，自动过滤 Release 消除双击跳
    pub fn next(&self) -> AppResult<Event> {
        loop {
            if event::poll(self.tick_rate)? {
                match event::read()? {
                    event::Event::Key(key) => {
                        // 只处理 Press 和 Repeat，跳过 Release
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }
                        return Ok(Event::Key(key));
                    }
                    event::Event::Resize(w, h) => return Ok(Event::Resize(w, h)),
                    _ => {}
                }
            } else {
                return Ok(Event::Tick);
            }
        }
    }
}
