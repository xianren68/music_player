// ─── 顶部栏模块 ───
use gpui::*;
use gpui_component::ActiveTheme as _;

/// 构建顶部栏：▶ 标题 + ⚙
pub fn build_topbar(
    sidebar_open: bool,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("topbar").flex().flex_row().items_center().flex_none().px_3().py_2()
        // 左侧：侧边栏开关
        .child(
            div().id("btn-side").text_color(cx.theme().muted_foreground).text_lg().cursor_pointer()
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_click(cx.listener(crate::MusicPlayer::toggle_sidebar))
                .child(if sidebar_open { "◀" } else { "▶" })
        )
        // 弹性空间
        .child(div().flex_1())
        // 标题
        .child(div().text_color(cx.theme().muted_foreground).text_sm().child("本地音乐播放器"))
        // 弹性空间
        .child(div().flex_1())
        // 右侧：+ 和 ⚙
        .child(
            div().id("btn-add").text_color(cx.theme().muted_foreground).text_lg().cursor_pointer()
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_click(cx.listener(crate::MusicPlayer::add_folder))
                .child("+")
        )
        .child(div().w(px(12.0)))
        .child(
            div().id("btn-set").text_color(cx.theme().muted_foreground).text_lg().cursor_pointer()
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_click(cx.listener(crate::MusicPlayer::toggle_settings))
                .child("⚙")
        )
        .into_any()
}
