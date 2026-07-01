// ─── 主题系统 ───
use gpui::{Hsla, SharedString};

/// 主题配置：所有可自定义的颜色和背景图
#[derive(Clone)]
pub struct ThemeConfig {
    /// 主题名称
    pub name: SharedString,
    /// 背景色
    pub bg: Hsla,
    /// 表面色（侧边栏、面板）
    pub surface: Hsla,
    /// 前景色（主要文字）
    pub fg: Hsla,
    /// 弱文字色
    pub muted: Hsla,
    /// 更弱文字色
    pub muted_fg: Hsla,
    /// 强调色（按钮、选中、进度条）
    pub accent: Hsla,
    /// 亮强调色（图标、高亮文字）
    pub accent_light: Hsla,
    /// 选中背景色
    pub active: Hsla,
    /// 悬停背景色
    pub hover: Hsla,
    /// 边框色
    pub border: Hsla,
    /// 播放按钮背景
    pub play_btn: Hsla,
    /// 播放按钮文字
    pub play_btn_fg: Hsla,
    /// 背景图片路径
    pub bg_image: Option<SharedString>,
}

impl ThemeConfig {
    /// 暗色主题预设
    pub fn dark() -> Self {
        Self {
            name: "暗色".into(),
            bg: hsla(240.0, 0.12, 0.04, 1.0),       // #0a0a0f
            surface: hsla(240.0, 0.15, 0.08, 1.0),   // #12121a
            fg: hsla(240.0, 0.20, 0.97, 1.0),        // #f5f5fa
            muted: hsla(240.0, 0.15, 0.63, 1.0),     // #a0a0b8
            muted_fg: hsla(240.0, 0.10, 0.38, 1.0),  // #5a5a72
            accent: hsla(252.0, 0.73, 0.64, 1.0),    // #6c5ce7
            accent_light: hsla(252.0, 0.67, 0.79, 1.0), // #a29bfe
            active: hsla(252.0, 0.40, 0.20, 1.0),
            hover: hsla(252.0, 0.15, 0.14, 1.0),
            border: hsla(240.0, 0.10, 0.14, 1.0),
            play_btn: hsla(0.0, 0.0, 1.0, 1.0),
            play_btn_fg: hsla(240.0, 0.12, 0.04, 1.0),
            bg_image: None,
        }
    }

    /// 亮色主题预设
    pub fn light() -> Self {
        Self {
            name: "亮色".into(),
            bg: hsla(240.0, 0.10, 0.97, 1.0),
            surface: hsla(240.0, 0.10, 0.93, 1.0),
            fg: hsla(240.0, 0.12, 0.08, 1.0),
            muted: hsla(240.0, 0.10, 0.40, 1.0),
            muted_fg: hsla(240.0, 0.08, 0.55, 1.0),
            accent: hsla(252.0, 0.73, 0.55, 1.0),
            accent_light: hsla(252.0, 0.67, 0.65, 1.0),
            active: hsla(252.0, 0.20, 0.88, 1.0),
            hover: hsla(252.0, 0.15, 0.90, 1.0),
            border: hsla(240.0, 0.08, 0.85, 1.0),
            play_btn: hsla(252.0, 0.73, 0.64, 1.0),
            play_btn_fg: hsla(0.0, 0.0, 1.0, 1.0),
            bg_image: None,
        }
    }

    /// 根据字符串 HSL 更新颜色
    pub fn with_bg(mut self, h: f32, s: f32, l: f32) -> Self {
        self.bg = hsla(h, s, l, 1.0);
        self
    }
    pub fn with_surface(mut self, h: f32, s: f32, l: f32) -> Self {
        self.surface = hsla(h, s, l, 1.0);
        self
    }
    pub fn with_fg(mut self, h: f32, s: f32, l: f32) -> Self {
        self.fg = hsla(h, s, l, 1.0);
        self
    }
    pub fn with_accent(mut self, h: f32, s: f32, l: f32) -> Self {
        self.accent = hsla(h, s, l, 1.0);
        self
    }
    pub fn with_bg_image(mut self, path: Option<SharedString>) -> Self {
        self.bg_image = path;
        self
    }
}

/// HSL 颜色构造辅助
fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}
