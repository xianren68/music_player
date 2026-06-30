// ─── 设置面板模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{ActiveTheme as _, slider::*, switch::*};

/// 构建设置面板
pub fn build_settings(
    opacity: f32,
    opacity_enabled: bool,
    follow_system_theme: bool,
    is_dark: bool,
    slider: &Entity<SliderState>,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("settings-panel").flex_none().w(px(280.0)).flex_col()
        .bg(cx.theme().background)
        // 标题栏
        .child(div().flex().items_center().justify_between().px_4().py_3()
            .border_b_1().border_color(cx.theme().border)
            .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(cx.theme().foreground).child("设置"))
            .child(div().id("hdr-close").text_color(cx.theme().muted_foreground).text_lg().cursor_pointer()
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_click(cx.listener(crate::MusicPlayer::toggle_settings))
                .child("×")))
        // 设置内容
        .child(div().flex_col().p_4().gap_8()
            // ── 主题设置 ──
            .child(
                div().flex_col().gap_4()
                    .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(cx.theme().foreground).child("主题"))
                    // 跟随系统开关
                    .child(
                        div().flex().items_center().justify_between()
                            .child(div().text_sm().text_color(cx.theme().foreground).child("跟随系统"))
                            .child(
                                Switch::new("follow-system")
                                    .checked(follow_system_theme)
                                    .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                                        this.follow_system_theme = *checked;
                                        if *checked {
                                            gpui_component::Theme::sync_system_appearance(None, cx);
                                        }
                                        cx.notify();
                                    }))
                            )
                    )
                    // 手动选择主题（仅在非跟随系统时显示）
                    .when(!follow_system_theme, |this| this.child(
                        div().flex().gap_3()
                            .child(
                                div().id("btn-light").flex_1().px_3().py_2().rounded_md().text_xs().cursor_pointer().text_center()
                                    .bg(if !is_dark { cx.theme().primary } else { cx.theme().secondary })
                                    .text_color(if !is_dark { cx.theme().primary_foreground } else { cx.theme().foreground })
                                    .hover(|s| s.opacity(0.8))
                                    .on_click(cx.listener(crate::MusicPlayer::set_light_theme))
                                    .child("亮色")
                            )
                            .child(
                                div().id("btn-dark").flex_1().px_3().py_2().rounded_md().text_xs().cursor_pointer().text_center()
                                    .bg(if is_dark { cx.theme().primary } else { cx.theme().secondary })
                                    .text_color(if is_dark { cx.theme().primary_foreground } else { cx.theme().foreground })
                                    .hover(|s| s.opacity(0.8))
                                    .on_click(cx.listener(crate::MusicPlayer::set_dark_theme))
                                    .child("暗色")
                            )
                    ))
            )

            // ── 透明度设置 ──
            .child(
                div().flex_col().gap_4()
                    .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(cx.theme().foreground).child("窗口透明度"))
                    // 透明度开关
                    .child(
                        div().flex().items_center().justify_between()
                            .child(div().text_sm().text_color(cx.theme().foreground).child("启用透明度"))
                            .child(
                                Switch::new("opacity-toggle")
                                    .checked(opacity_enabled)
                                    .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                                        this.opacity_enabled = *checked;
                                        if !*checked {
                                            this.opacity = 1.0;
                                        }
                                        cx.notify();
                                    }))
                            )
                    )
                    // 透明度滑块（仅在启用时显示）
                    .when(opacity_enabled, |this| this.child(
                        div().flex_col().gap_2()
                            // 标签行：标题 + 百分比
                            .child(
                                div().flex().items_center().justify_between()
                                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("背景不透明度"))
                                    .child(div().text_xs().text_color(cx.theme().foreground).font_weight(gpui::FontWeight::BOLD)
                                        .child(format!("{:.0}%", opacity * 100.0)))
                            )
                            .child(Slider::new(slider))
                    ))
            )
        )
        .into_any()
}
