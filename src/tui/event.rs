use std::time::Duration;

use crate::error::AppResult;
use crossterm::event::{self, KeyEvent};

/// 事件类型
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

/// 事件处理器 —— 封装 crossterm 事件轮询
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// 获取下一个事件（阻塞）
    pub fn next(&self) -> AppResult<Event> {
        loop {
            if event::poll(self.tick_rate)? {
                match event::read()? {
                    event::Event::Key(key) => return Ok(Event::Key(key)),
                    event::Event::Resize(w, h) => return Ok(Event::Resize(w, h)),
                    _ => {}
                }
            } else {
                // 超时 → 触发 Tick
                return Ok(Event::Tick);
            }
        }
    }
}
