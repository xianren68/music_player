// ─── 设置面板模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{slider::*, switch::*};
use crate::theme::ThemeConfig;

/// 设置项图标背景
fn setting_icon_bg(t: &ThemeConfig) -> Hsla {
    Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.12 }
}

pub fn build_settings(
    opacity: f32,
    opacity_enabled: bool,
    slider: &Entity<SliderState>,
    t: &ThemeConfig,
    theme_mode: crate::ThemeMode,
    bg_slider: &Entity<SliderState>,
    bg_blur: bool,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    let has_bg = t.bg_image.is_some();
    let bg_content_opacity = bg_slider.read(cx).value().start();
    let is_dark = theme_mode == crate::ThemeMode::Dark;
    let music_folder: SharedString = crate::settings::AppSettings::load()
        .music_folder.clone().unwrap_or_default().into();

    div().id("settings-panel").flex_none().w(px(300.0)).flex_col()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.4 })
        .border_l_1().border_color(t.border)
        // ── 标题栏 ──
        .child(
            div().px_5().pt_5().pb_4().flex().items_center().gap_2()
                .child(svg().path("icons/settings.svg").size_5().text_color(t.accent_light))
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).text_color(t.fg).child("设置"))
        )
        // ── 内容区 ──
        .child(div().flex_col().py_2().gap_1().overflow_hidden()
            // ── 外观分组 ──
            .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.muted_fg).px_5().mt_5().mb_1().child("外观"))
            // 暗色模式 (Switch)
            .child(div().flex().items_center().gap_3().px_5().py_2()
                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                    .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                    .child(svg().path("icons/palette.svg").size_4().text_color(t.accent_light)))
                .child(div().flex_1().flex_col()
                    .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("暗色模式"))
                    .child(div().text_xs().text_color(t.muted_fg).child("始终开启")))
                .child(Switch::new("theme-toggle").checked(is_dark)
                    .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                        if *checked {
                            this.theme = ThemeConfig::dark();
                            this.theme_mode = crate::ThemeMode::Dark;
                        } else {
                            this.theme = ThemeConfig::light();
                            this.theme_mode = crate::ThemeMode::Light;
                        }
                        this.save_settings(cx);
                        cx.notify();
                    }))))
            // 窗口透明度
            .child(div().flex().items_center().gap_3().px_5().py_2()
                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                    .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                    .child(svg().path("icons/eye.svg").size_4().text_color(t.accent_light)))
                .child(div().flex_1().flex_col()
                    .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("窗口透明度"))
                    .child(div().text_xs().text_color(t.muted_fg).child("启用后窗口背景透明")))
                .child(Switch::new("opacity-toggle").checked(opacity_enabled)
                    .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                        this.opacity_enabled = *checked;
                        this.save_settings(cx);
                        cx.notify();
                    }))))
            .when(opacity_enabled, |this| this.child(
                div().flex_col().gap_2().px_5().ml_9()
                    .child(div().flex().items_center().justify_between()
                        .child(div().text_xs().text_color(t.muted_fg).child("不透明度"))
                        .child(div().text_xs().text_color(t.fg).font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{:.0}%", opacity * 100.0))))
                    .child(Slider::new(slider))))

            // ── 背景图分组 ──
            .child(div().mt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.muted_fg).px_5().mb_1().child("背景图"))
            // 选择/清除按钮
            .child(div().flex().gap_2().px_5()
                .child(
                    div().id("btn-bg-pick").flex_1().px_3().py_2().rounded_lg().text_xs()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }).text_color(t.fg)
                        .border_1().border_color(t.border)
                        .flex().items_center().justify_center().gap_2()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                        .on_click(cx.listener(crate::MusicPlayer::pick_bg_image))
                        .child(svg().path("icons/folder.svg").size_4().text_color(t.muted_fg))
                        .child("选择图片")
                )
                .child(
                    div().id("btn-bg-clear").px_3().py_2().rounded_lg().text_xs()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }).text_color(t.muted_fg)
                        .border_1().border_color(t.border)
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                        .on_click(cx.listener(crate::MusicPlayer::clear_bg_image))
                        .child("清除")
                )
            )
            .when(has_bg, |this| this
                // 磨砂效果
                .child(div().flex().items_center().gap_3().px_5().py_2()
                    .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                    .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                        .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                        .child(svg().path("icons/snowflake.svg").size_4().text_color(t.accent_light)))
                    .child(div().flex_1().flex_col()
                        .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("磨砂效果"))
                        .child(div().text_xs().text_color(t.muted_fg).child("在背景图上叠加模糊层")))
                    .child(Switch::new("blur-toggle").checked(bg_blur)
                        .on_click(cx.listener(crate::MusicPlayer::toggle_bg_blur))))
                // 背景图透明度
                .child(div().flex_col().gap_2().px_5().ml_9()
                    .child(div().flex().items_center().justify_between()
                        .child(div().text_xs().text_color(t.muted_fg).child("背景图透明度"))
                        .child(div().text_xs().text_color(t.fg).font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{:.0}%", bg_content_opacity * 100.0))))
                    .child(Slider::new(bg_slider)))
                // 文件路径
                .child(div().px_5().text_xs().text_color(t.muted_fg).text_ellipsis()
                    .child(t.bg_image.clone().unwrap_or_default().to_string()))
            )

            // ── 资料库分组 ──
            .child(div().mt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(t.muted_fg).px_5().mb_1().child("资料库"))
            // 音乐文件夹
            .child(div().flex().items_center().gap_3().px_5().py_2()
                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                    .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                    .child(svg().path("icons/folder.svg").size_4().text_color(t.accent_light)))
                .child(div().flex_1().flex_col()
                    .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("音乐文件夹"))
                    .child(div().text_xs().text_color(t.muted_fg).text_ellipsis().child(music_folder)))
                .child(
                    div().id("btn-music-pick").px_3().py_2().rounded_lg().text_xs()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }).text_color(t.fg)
                        .border_1().border_color(t.border)
                        .flex().items_center().justify_center().gap_2()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                        .on_click(cx.listener(crate::MusicPlayer::pick_music_folder))
                        .child(svg().path("icons/folder.svg").size_4().text_color(t.muted_fg))
                        .child("选择")
                )
            )
        )
        .into_any()
}
