// ─── 中间内容模块 ───
use gpui::*;
use gpui_component::ActiveTheme as _;

/// 构建中间内容区（专辑封面 + 曲目信息）
/// 使用绝对定位填满 topbar(48px) 和 player_bar(140px) 之间的空间
pub fn build_center(
    title: &str,
    artist: &str,
    album: &str,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("center")
        .absolute().top(px(48.0)).bottom(px(140.0)).left(px(0.0)).right(px(0.0))
        .flex().flex_col().items_center().justify_center()
        // 专辑封面
        .child(
            div().id("art").flex_none().w(px(220.0)).h(px(220.0)).rounded_2xl()
                .bg(cx.theme().list).flex().items_center().justify_center().shadow_lg()
                .child(div().text_size(px(60.0)).text_color(cx.theme().muted_foreground).child("♪"))
        )
        // 曲名
        .child(div().mt_6().text_xl().font_weight(gpui::FontWeight::BOLD).text_color(cx.theme().foreground).child(title.to_string()))
        // 歌手
        .child(div().mt_2().text_base().text_color(cx.theme().muted).child(artist.to_string()))
        // 专辑
        .child(div().mt_1().text_sm().text_color(cx.theme().muted_foreground).child(album.to_string()))
        .into_any()
}
