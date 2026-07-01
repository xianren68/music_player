// ─── 硬编码颜色方案（暗色主题）───
use gpui::Hsla;

/// 背景色（深灰）
pub fn bg() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.12, a: 1.0 }
}

/// 侧边栏/面板背景色
pub fn surface() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.15, a: 1.0 }
}

/// 前景色（白色文字）
pub fn fg() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 1.0, a: 1.0 }
}

/// 弱化前景色（灰色文字）
pub fn muted() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.6, a: 1.0 }
}

/// 更弱的文字色
pub fn muted_fg() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.45, a: 1.0 }
}

/// 强调色（蓝紫色）
pub fn accent() -> Hsla {
    Hsla { h: 250.0, s: 0.7, l: 0.65, a: 1.0 }
}

/// 选中的列表项背景
pub fn list_active() -> Hsla {
    Hsla { h: 250.0, s: 0.5, l: 0.25, a: 1.0 }
}

/// 按钮悬停背景
pub fn hover() -> Hsla {
    Hsla { h: 250.0, s: 0.3, l: 0.2, a: 1.0 }
}

/// 边框色
pub fn border() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.2, a: 1.0 }
}

/// 播放按钮背景（白色）
pub fn play_btn_bg() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 1.0, a: 1.0 }
}

/// 播放按钮文字（深色）
pub fn play_btn_fg() -> Hsla {
    Hsla { h: 0.0, s: 0.0, l: 0.12, a: 1.0 }
}
