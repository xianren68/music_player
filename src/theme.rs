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
            bg: hsla(250.0, 0.25, 0.06, 1.0),       // 深紫黑背景
            surface: hsla(250.0, 0.20, 0.10, 1.0),   // 表面
            fg: hsla(0.0, 0.0, 0.95, 1.0),           // 前景白
            muted: hsla(250.0, 0.15, 0.50, 1.0),     // 弱文字
            muted_fg: hsla(250.0, 0.10, 0.35, 1.0),  // 更弱
            accent: hsla(265.0, 1.0, 0.65, 1.0),     // 纯紫
            accent_light: hsla(265.0, 1.0, 0.82, 1.0), // 亮紫
            active: hsla(265.0, 0.60, 0.22, 1.0),
            hover: hsla(265.0, 0.30, 0.15, 1.0),
            border: hsla(250.0, 0.15, 0.15, 1.0),
            play_btn: hsla(0.0, 0.0, 0.95, 1.0),
            play_btn_fg: hsla(250.0, 0.25, 0.06, 1.0),
            bg_image: None,
        }
    }

    /// 亮色主题预设
    pub fn light() -> Self {
        Self {
            name: "亮色".into(),
            bg: hsla(250.0, 0.15, 0.96, 1.0),
            surface: hsla(250.0, 0.12, 0.92, 1.0),
            fg: hsla(250.0, 0.10, 0.10, 1.0),
            muted: hsla(250.0, 0.10, 0.45, 1.0),
            muted_fg: hsla(250.0, 0.08, 0.55, 1.0),
            accent: hsla(265.0, 1.0, 0.55, 1.0),
            accent_light: hsla(265.0, 1.0, 0.65, 1.0),
            active: hsla(265.0, 0.30, 0.88, 1.0),
            hover: hsla(265.0, 0.15, 0.90, 1.0),
            border: hsla(250.0, 0.10, 0.85, 1.0),
            play_btn: hsla(265.0, 1.0, 0.55, 1.0),
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
