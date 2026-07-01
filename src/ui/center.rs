// ─── 中间内容模块 ───
use gpui::*;
use crate::theme::ThemeConfig;

pub fn build_center(
    title: &str,
    artist: &str,
    _album: &str,
    t: &ThemeConfig,
    _cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("center")
        .absolute().top(px(48.0)).bottom(px(140.0)).left(px(0.0)).right(px(0.0))
        .flex().flex_col().items_center().justify_center()
        .child(
            // ── NOW PLAYING 标签 ──
            div().flex().items_center().gap_2().mb_4()
                .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(t.accent_light))
                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.accent_light)
                    .child("正在播放"))
        )
        .child(
            // ── 专辑封面 ──
            div().id("art").flex_none().w(px(240.0)).h(px(240.0)).rounded_2xl()
                .bg(t.surface).flex().items_center().justify_center()
                .shadow_2xl()
                .child(div().text_size(px(64.0)).text_color(t.muted_fg).child("♪"))
        )
        .child(
            // ── 曲目信息 ──
            div().flex_col().items_center().mt_6()
                .child(div().text_2xl().font_weight(gpui::FontWeight::BOLD)
                    .text_color(t.fg)
                    .child(title.to_string()))
                .child(div().text_base().text_color(t.muted).mt_1()
                    .child(artist.to_string()))
        )
        .into_any()
}
