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
        .flex().flex_col().items_center()
        // 参考截图：封面 + 歌曲信息 + 进度条 + 控制按钮 整体垂直居中
        .justify_center()
        // ── NOW PLAYING 标签 ──
        .child(
            div().flex().items_center().gap_2().mb_4()
                .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(t.accent_light))
                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.accent_light)
                    .child("NOW PLAYING"))
        )
        // ── 专辑封面 ──
        .child(
            div().id("art").flex_none().w(px(240.0)).h(px(240.0)).rounded_2xl()
                .bg(t.surface).flex().items_center().justify_center()
                .shadow_2xl()
                .child(div().text_size(px(64.0)).text_color(t.muted_fg).child("♪"))
        )
        // ── 曲目信息 ──
        .child(
            div().flex_col().items_center().mt_5()
                .child(div().text_2xl().font_weight(gpui::FontWeight::BOLD)
                    .text_color(t.fg)
                    .child(title.to_string()))
                .child(div().text_base().text_color(t.muted).mt_1()
                    .child(artist.to_string()))
        )
        .into_any()
}
