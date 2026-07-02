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
    /// 暗色主题预设（参考 music-player.html 配色）
    ///
    /// ⚠️ 重要：GPUI 的 Hsla.h 范围是 0~1（不是 0~360），
    /// 需要将 CSS 色相度数除以 360。
    /// 例：CSS h=247° → GPUI h=247/360≈0.686
    pub fn dark() -> Self {
        Self {
            name: "暗色".into(),
            // --bg-base: #0a0a0f → CSS-HSL h=240° s=0.20 l=0.05
            bg: css_hsl(240.0, 0.20, 0.05),
            // --bg-surface: #12121a → CSS-HSL h=240° s=0.18 l=0.09
            surface: css_hsl(240.0, 0.18, 0.09),
            // --text-primary: #f5f5fa
            fg: css_hsl(240.0, 0.32, 0.97),
            // --text-secondary: #a0a0b8
            muted: css_hsl(240.0, 0.15, 0.67),
            // --text-muted: #5a5a72
            muted_fg: css_hsl(240.0, 0.12, 0.40),
            // --accent: #6c5ce7 → CSS h≈247°
            accent: css_hsl(247.0, 0.74, 0.63),
            // --accent-light: #a29bfe → CSS h≈244°
            accent_light: css_hsl(244.0, 0.98, 0.80),
            // 选中背景: rgba(108,92,231,0.12)
            active: css_hsla(247.0, 0.74, 0.63, 0.12),
            // 悬停背景: rgba(255,255,255,0.04)
            hover: css_hsla(0.0, 0.0, 1.0, 0.04),
            // --border-subtle
            border: css_hsla(0.0, 0.0, 1.0, 0.06),
            // 播放按钮: 白色
            play_btn: css_hsl(240.0, 0.32, 0.97),
            // 播放按钮图标: 深色
            play_btn_fg: css_hsl(240.0, 0.25, 0.06),
            bg_image: None,
        }
    }

    /// 亮色主题预设
    /// 保持与暗色主题一致的紫色强调色，背景调整为浅色
    pub fn light() -> Self {
        Self {
            name: "亮色".into(),
            // 背景: 浅灰白 #f5f5fa
            bg: css_hsl(240.0, 0.32, 0.97),
            // 表面色: 稍深一点 #ededef
            surface: css_hsl(240.0, 0.10, 0.92),
            // 前景色: 深近黑 #0a0a0f
            fg: css_hsl(240.0, 0.20, 0.05),
            // 弱文字色: 中灰 #5a5a72
            muted: css_hsl(240.0, 0.12, 0.40),
            // 更弱文字色: 浅灰 #8a8a9a
            muted_fg: css_hsl(240.0, 0.08, 0.58),
            // --accent: #6c5ce7 (与暗色主题一致)
            accent: css_hsl(247.0, 0.74, 0.63),
            // --accent-light: #a29bfe (与暗色主题一致)
            accent_light: css_hsl(244.0, 0.98, 0.80),
            // 选中背景: rgba(108,92,231,0.12)
            active: css_hsla(247.0, 0.74, 0.63, 0.12),
            // 悬停背景: rgba(0,0,0,0.04)
            hover: css_hsla(0.0, 0.0, 0.0, 0.04),
            // 边框色: rgba(0,0,0,0.08)
            border: css_hsla(0.0, 0.0, 0.0, 0.08),
            // 播放按钮: 紫色 #6c5ce7
            play_btn: css_hsl(247.0, 0.74, 0.63),
            // 播放按钮图标: 白色
            play_btn_fg: css_hsla(0.0, 0.0, 1.0, 1.0),
            bg_image: None,
        }
    }

    pub fn with_bg(mut self, h: f32, s: f32, l: f32) -> Self {
        self.bg = Hsla { h, s, l, a: 1.0 };
        self
    }
    pub fn with_surface(mut self, h: f32, s: f32, l: f32) -> Self {
        self.surface = Hsla { h, s, l, a: 1.0 };
        self
    }
    pub fn with_fg(mut self, h: f32, s: f32, l: f32) -> Self {
        self.fg = Hsla { h, s, l, a: 1.0 };
        self
    }
    pub fn with_accent(mut self, h: f32, s: f32, l: f32) -> Self {
        self.accent = Hsla { h, s, l, a: 1.0 };
        self
    }
    pub fn with_bg_image(mut self, path: Option<SharedString>) -> Self {
        self.bg_image = path;
        self
    }
}

/// 从 CSS HSL 值创建 GPUI Hsla
/// CSS 使用 h: 0~360°, s: 0~1, l: 0~1
/// GPUI 使用 h: 0~1,   s: 0~1, l: 0~1 （内部会做 h*6 取模）
fn css_hsl(h_deg: f32, s: f32, l: f32) -> Hsla {
    Hsla { h: h_deg / 360.0, s, l, a: 1.0 }
}

/// 带 alpha 的版本
fn css_hsla(h_deg: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h: h_deg / 360.0, s, l, a }
}
