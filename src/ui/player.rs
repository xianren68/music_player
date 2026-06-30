// ─── 播放控制栏模块 ───
use gpui::*;
use gpui_component::ActiveTheme as _;

/// 构建底部播放控制栏（绝对定位贴底）
pub fn build_player_bar(
    playing: bool,
    progress: f64,
    cur_t: f64,
    tot_t: f64,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("bar")
        .absolute().bottom(px(0.0)).left(px(0.0)).right(px(0.0))
        .flex_col().px_6().pt_3().pb_5()
        .border_t_1().border_color(cx.theme().border)
        // 进度条
        .child(
            div().id("prog-track").w_full().h(px(3.0)).rounded_full().bg(cx.theme().border).relative()
                .child(
                    div().id("prog-fill").absolute().top(px(0.0)).left(px(0.0))
                        .h(px(3.0)).rounded_full().bg(cx.theme().primary)
                        .w(px(progress.max(0.01) as f32 * 500.0))
                )
        )
        // 时间显示
        .child(
            div().flex().justify_between().w_full().mt_2()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child(crate::audio::fmt_time(cur_t)))
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child(crate::audio::fmt_time(tot_t)))
        )
        // 控制按钮
        .child(
            div().flex().items_center().justify_center().gap_8().mt_3()
                // 上一曲
                .child(
                    div().id("c-prev").w(px(36.0)).h(px(36.0)).rounded_full().bg(cx.theme().list)
                        .hover(|s| s.bg(cx.theme().accent))
                        .active(|s| s.opacity(0.8))
                        .cursor_pointer()
                        .flex().items_center().justify_center().text_color(cx.theme().muted_foreground)
                        .on_click(cx.listener(crate::MusicPlayer::prev))
                        .child("◀◀")
                )
                // 播放/暂停
                .child(
                    div().id("c-play").w(px(48.0)).h(px(48.0)).rounded_full().bg(cx.theme().foreground)
                        .hover(|s| s.opacity(0.9))
                        .active(|s| s.opacity(0.8))
                        .cursor_pointer()
                        .flex().items_center().justify_center().text_lg().text_color(cx.theme().background)
                        .on_click(cx.listener(crate::MusicPlayer::play_pause))
                        .child(if playing { "⏸" } else { "▶" })
                )
                // 下一曲
                .child(
                    div().id("c-next").w(px(36.0)).h(px(36.0)).rounded_full().bg(cx.theme().list)
                        .hover(|s| s.bg(cx.theme().accent))
                        .active(|s| s.opacity(0.8))
                        .cursor_pointer()
                        .flex().items_center().justify_center().text_color(cx.theme().muted_foreground)
                        .on_click(cx.listener(crate::MusicPlayer::next))
                        .child("▶▶")
                )
        )
        .into_any()
}
