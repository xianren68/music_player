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
    shuffle: bool,
    repeat: crate::RepeatMode,
    volume_slider: &Entity<SliderState>,
    volume: f32,
    assoc_msg: Option<String>,
    settings_width: gpui::Pixels,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    let has_bg = t.bg_image.is_some();
    let bg_content_opacity = bg_slider.read(cx).value().start();
    let is_dark = theme_mode == crate::ThemeMode::Dark;

    // 获取音乐文件夹列表
    let music_folders: Vec<SharedString> = crate::settings::AppSettings::load()
        .music_folders.iter().map(|s| s.clone().into()).collect();

    div().id("settings-panel").flex_none().w(settings_width).flex_col()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.4 })
        .border_l_1().border_color(t.border)
        .child(
            div().px_5().pt_5().pb_2().flex().items_center().gap_2()
                .child(svg().path("icons/settings.svg").size_5().text_color(t.accent_light))
                .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).text_color(t.fg).child("设置"))
        )
        .child(
            div().flex_col().gap_1().overflow_hidden()
                // 播放分组标题
                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().mt_3().mb_1().child("播放"))

                // 随机播放
                .child(
                    div().flex().items_center().gap_3().px_5().py_2()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                        .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                            .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                            .child(svg().path("icons/shuffle.svg").size_4().text_color(t.accent_light)))
                        .child(div().flex_1().flex_col()
                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("随机播放"))
                            .child(div().text_xs().text_color(t.muted_fg)
                                .child(if shuffle { "已开启" } else { "按顺序播放" })))
                        .child(
                            Switch::new("shuffle-toggle").checked(shuffle)
                                .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                                    this.shuffle = *checked;
                                    this.save_settings(cx);
                                    cx.notify();
                                }))
                        )
                )

                // 循环模式：点右侧按钮在 顺序 / 列表循环 / 单曲循环 之间切换
                .child(
                    div().flex().items_center().gap_3().px_5().py_2()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                        .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                            .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                            .child(svg()
                                .path(if repeat == crate::RepeatMode::One {
                                    "icons/repeat-one.svg"
                                } else {
                                    "icons/repeat.svg"
                                })
                                .size_4().text_color(t.accent_light)))
                        .child(div().flex_1().flex_col()
                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("循环模式"))
                            .child(div().text_xs().text_color(t.muted_fg)
                                .child(match repeat {
                                    crate::RepeatMode::Off => "播完最后一首就停",
                                    crate::RepeatMode::All => "播完最后一首回到第一首",
                                    crate::RepeatMode::One => "单曲反复播放",
                                })))
                        .child(
                            div().id("btn-repeat").flex_none().px_3().py_1_5().rounded_lg()
                                .bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.12 })
                                .text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(t.accent_light)
                                .cursor_pointer()
                                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                                .on_click(cx.listener(crate::MusicPlayer::cycle_repeat))
                                .child(repeat.label())
                        )
                )

                // 音量
                .child(
                    div().flex().items_center().gap_3().px_5().py_2()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                        .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                            .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                            .child(svg().path("icons/volume.svg").size_4().text_color(t.accent_light)))
                        .child(div().flex_1().flex_col()
                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("音量"))
                            .child(div().text_xs().text_color(t.muted_fg)
                                .child(if volume <= 0.001 { "静音".to_string() } else { format!("{:.0}%", volume * 100.0) })))
                        .child(
                            div().id("btn-mute").flex_none().w(px(28.0)).h(px(28.0)).rounded_lg()
                                .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
                                .flex().items_center().justify_center()
                                .cursor_pointer()
                                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                                .on_click(cx.listener(|this, _e, w, cx| {
                                    // 静音 ⇄ 回到 50%（不用记上一次的值，够用且简单）
                                    let target = if this.volume <= 0.001 { 0.5 } else { 0.0 };
                                    this.volume = target;
                                    // 滑块也要跟着跳，否则 UI 上的圆点还停在旧位置
                                    this.volume_slider.update(cx, |s, cx2| {
                                        s.set_value(target as f32, w, cx2);
                                    });
                                    this.player.set_volume(target);
                                    this.save_settings(cx);
                                    cx.notify();
                                }))
                                .child(svg().path("icons/volume.svg").size_3_5().text_color(t.muted_fg))
                        )
                )

                // 音量滑块
                .child(
                    div().flex_col().gap_2().px_5().ml_9()
                        .child(Slider::new(volume_slider))
                )

                // 外观分组标题
                .child(div().pt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().pb_1().child("外观"))

                // 暗色模式
                .child(
                    div().flex().items_center().gap_3().px_5().py_2()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                        .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                            .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                            .child(svg().path("icons/palette.svg").size_4().text_color(t.accent_light)))
                        .child(div().flex_1().flex_col()
                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("暗色模式"))
                            .child(div().text_xs().text_color(t.muted_fg).child("始终开启")))
                        .child(
                            Switch::new("theme-toggle").checked(is_dark)
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
                                }))
                        )
                )

                // 窗口透明度
                .child(
                    div().flex().items_center().gap_3().px_5().py_2()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                        .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                            .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                            .child(svg().path("icons/eye.svg").size_4().text_color(t.accent_light)))
                        .child(div().flex_1().flex_col()
                            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("窗口透明度"))
                            .child(div().text_xs().text_color(t.muted_fg).child("启用后窗口背景透明")))
                        .child(
                            Switch::new("opacity-toggle").checked(opacity_enabled)
                                .on_click(cx.listener(|this, checked: &bool, _w: &mut Window, cx: &mut Context<crate::MusicPlayer>| {
                                    this.opacity_enabled = *checked;
                                    this.save_settings(cx);
                                    cx.notify();
                                }))
                        )
                )

                // 透明度滑块
                .when(opacity_enabled, |this| this.child(
                    div().flex_col().gap_2().px_5().ml_9()
                        .child(div().flex().items_center().justify_between()
                            .child(div().text_xs().text_color(t.muted_fg).child("不透明度"))
                            .child(div().text_xs().text_color(t.fg).font_weight(gpui::FontWeight::BOLD)
                                .child(format!("{:.0}%", opacity * 100.0))))
                        .child(Slider::new(slider))
                ))

                // 背景图分组标题
                .child(div().pt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().pb_1().child("背景图"))

                // 选择/清除背景图按钮
                .child(
                    div().flex().gap_2().px_5()
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

                // 背景图设置（如果已选择）
                .when(has_bg, |this| this
                    .child(
                        div().flex().items_center().gap_3().px_5().py_2()
                            .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.03 }))
                            .child(div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                                .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                                .child(svg().path("icons/snowflake.svg").size_4().text_color(t.accent_light)))
                            .child(div().flex_1().flex_col()
                                .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child("磨砂效果"))
                                .child(div().text_xs().text_color(t.muted_fg).child("在背景图上叠加模糊层")))
                            .child(
                                Switch::new("blur-toggle").checked(bg_blur)
                                    .on_click(cx.listener(crate::MusicPlayer::toggle_bg_blur))
                            )
                    )
                    .child(
                        div().flex_col().gap_2().px_5().ml_9()
                            .child(div().flex().items_center().justify_between()
                                .child(div().text_xs().text_color(t.muted_fg).child("背景图透明度"))
                                .child(div().text_xs().text_color(t.fg).font_weight(gpui::FontWeight::BOLD)
                                    .child(format!("{:.0}%", bg_content_opacity * 100.0))))
                            .child(Slider::new(bg_slider))
                    )
                    .child(
                        div().px_5().text_xs().text_color(t.muted_fg).text_ellipsis()
                            .child(t.bg_image.clone().unwrap_or_default().to_string())
                    )
                )

                // 资料库分组标题
                .child(div().pt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().pb_1().child("资料库"))

                // 添加音乐文件夹按钮
                .child(
                    div().id("btn-add-folder").mx_5().mt_2().px_3().py_2().rounded_lg().text_xs()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }).text_color(t.fg)
                        .border_1().border_color(t.border)
                        .flex().items_center().justify_center().gap_2()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.08 }))
                        .on_click(cx.listener(crate::MusicPlayer::pick_music_folder))
                        .child(svg().path("icons/folder.svg").size_4().text_color(t.accent_light))
                        .child("添加音乐文件夹")
                )

                // 文件夹列表
                .children({
                    let folder_els: Vec<AnyElement> = music_folders.into_iter().enumerate().map(|(idx, folder_path)| {                        let folder_path_clone = folder_path.clone();
                        let folder_path_clone2 = folder_path.clone();
                        let folder_name = std::path::Path::new(folder_path.as_ref())
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("未知")
                            .to_string();

                        div().mx_5().mt_2()
                            .rounded_lg()
                            .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.04 })
                            .border_1().border_color(t.border)
                            .px_3().py_2()
                            .flex().items_center().gap_2()
                            .child(
                                div().flex_none().w(px(28.0)).h(px(28.0)).rounded_lg()
                                    .bg(setting_icon_bg(t)).flex().items_center().justify_center()
                                    .child(svg().path("icons/folder.svg").size_3_5().text_color(t.accent_light))
                            )
                            .child(
                                div().flex_1().flex_col().min_w(px(0.0))
                                    .child(div().text_xs().font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg).child(folder_name))
                                    .child(div().text_xs().text_color(t.muted_fg).text_ellipsis().child(folder_path.clone()))
                            )
                            .child(
                                div().flex().gap_1()
                                    .child(
                                        div().id(("btn-refresh", idx as u64)).w(px(28.0)).h(px(28.0)).rounded_lg()
                                            .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
                                            .flex().items_center().justify_center()
                                            .cursor_pointer()
                                            .hover(|style| style.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.1 }))
                                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                                this.refresh_music_folder(folder_path_clone.clone(), cx);
                                            }))
                                            .child(svg().path("icons/refresh.svg").size_3_5().text_color(t.muted_fg))
                                    )
                                    .child(
                                        div().id(("btn-delete", idx as u64)).w(px(28.0)).h(px(28.0)).rounded_lg()
                                            .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
                                            .flex().items_center().justify_center()
                                            .cursor_pointer()
                                            .hover(|style| style.bg(Hsla { h: 0.0, s: 0.8, l: 0.5, a: 0.15 }))
                                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                                this.remove_music_folder(folder_path_clone2.clone(), cx);
                                            }))
                                            .child(svg().path("icons/trash.svg").size_3_5().text_color(t.muted_fg))
                                    )
                            )
                            .into_any()
                    }).collect();
                    folder_els
                })

                // 系统分组标题（第四组：托盘 / 文件关联）
                .child(div().pt_5().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().pb_1().child("系统"))

                // 文件关联：写 HKCU 注册表，把本程序加进音频文件的"打开方式"
                .child(
                    div().id("btn-assoc").mx_5().mt_2().px_3().py_2().rounded_lg().text_xs()
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }).text_color(t.fg)
                        .border_1().border_color(t.border)
                        .flex().items_center().justify_center().gap_2()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.08 }))
                        .on_click(cx.listener(crate::MusicPlayer::register_association))
                        .child(svg().path("icons/music.svg").size_4().text_color(t.accent_light))
                        .child("注册文件关联")
                )
                .child(
                    div().mx_5().mt_2().text_xs().text_color(t.muted_fg)
                        .child(assoc_msg.unwrap_or_else(|| {
                            "右键音频文件 → 打开方式里会出现本程序（不修改系统默认程序）".to_string()
                        }))
                )
                .child(div().h(px(16.0)))
        )
        .into_any()
}
