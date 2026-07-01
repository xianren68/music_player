// ─── 顶部标题栏模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use crate::theme::ThemeConfig;

pub fn build_topbar(
    sidebar_open: bool,
    settings_open: bool,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("topbar").flex().flex_row().items_center().flex_none()
        .h(px(40.0)).px_3()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.6 })
        .border_b_1().border_color(t.border)
        // ── 左侧：应用名称 ──
        .child(div().flex().items_center().gap_2()
            .child(svg().path("icons/music.svg").size_4().text_color(t.accent_light))
            .child(div().text_xs().text_color(t.muted).child("本地音乐播放器")))
        .child(div().flex_1())
        // ── 右侧：控制按钮 ──
        .child(div().flex().items_center().gap_0p5()
            // 切换音乐库
            .child(
                div().id("btn-toggle-lib").w(px(32.0)).h(px(28.0)).rounded_md()
                    .flex().items_center().justify_center()
                    .text_color(if sidebar_open { t.accent_light } else { t.muted_fg })
                    .when(sidebar_open, |this| this.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.18 }))
                    .cursor_pointer()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                    .on_click(cx.listener(crate::MusicPlayer::toggle_sidebar))
                    .child(svg().path("icons/list.svg").size_4()
                        .text_color(if sidebar_open { t.accent_light } else { t.muted_fg }))
            )
            // 切换设置
            .child(
                div().id("btn-toggle-set").w(px(32.0)).h(px(28.0)).rounded_md()
                    .flex().items_center().justify_center()
                    .text_color(if settings_open { t.accent_light } else { t.muted_fg })
                    .when(settings_open, |this| this.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.18 }))
                    .cursor_pointer()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                    .on_click(cx.listener(crate::MusicPlayer::toggle_settings))
                    .child(svg().path("icons/settings.svg").size_4()
                        .text_color(if settings_open { t.accent_light } else { t.muted_fg }))
            )
            // 最小化
            .child(
                div().id("btn-min").w(px(36.0)).h(px(28.0)).rounded_md()
                    .flex().items_center().justify_center()
                    .text_color(t.muted_fg)
                    .cursor_pointer()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }).text_color(t.fg))
                    .child(div().text_sm().text_color(t.muted_fg).child("−"))
            )
            // 全屏
            .child(
                div().id("btn-max").w(px(36.0)).h(px(28.0)).rounded_md()
                    .flex().items_center().justify_center()
                    .text_color(t.muted_fg)
                    .cursor_pointer()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }).text_color(t.fg))
                    .child(svg().path("icons/fullscreen.svg").size_4().text_color(t.muted_fg))
            )
            // 关闭
            .child(
                div().id("btn-close").w(px(44.0)).h(px(28.0)).rounded_md()
                    .flex().items_center().justify_center()
                    .text_color(t.muted_fg)
                    .cursor_pointer()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.69, l: 0.47, a: 1.0 }).text_color(rgb(0xffffff)))
                    .child(svg().path("icons/close.svg").size_4().text_color(t.muted_fg))
            )
        )
        .into_any()
}
