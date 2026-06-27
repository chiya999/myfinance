use ratatui::style::Color;

/// 主题配色方案
#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub text: Color,
    pub muted: Color,
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Rgb(64, 150, 255),
            success: Color::Rgb(80, 200, 120),
            danger: Color::Rgb(255, 80, 80),
            warning: Color::Rgb(255, 200, 50),
            text: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(120, 120, 120),
            bg: Color::Rgb(20, 22, 28),
            surface: Color::Rgb(30, 33, 40),
            border: Color::Rgb(60, 63, 70),
            highlight: Color::Rgb(70, 70, 90),
        }
    }
}
