// ─── 迷你播放器（第四组）───
// 窗口缩到 420×120 时的紧凑单行布局：封面 + 歌名/歌手 + 细进度条 + 控制按钮。
// 进入/退出由 main.rs 的 toggle_mini_mode 负责（那边还要改窗口尺寸），这里只管画。
use gpui::*;
use std::sync::Arc;
use crate::theme::ThemeConfig;

pub fn build_mini_player(
    playing: bool,
    cur_t: f64,
    tot_t: f64,
    title: &str,
    artist: &str,
    cover: Option<&Arc<RenderImage>>,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 进度比例（0~1），只用来画填充条，不接拖拽
    let pct = if tot_t > 0.0 { (cur_t / tot_t).clamp(0.0, 1.0) as f32 } else { 0.0 };

    // 封面：64×64，没封面就用音符占位
    let art = div().flex_none().w(px(64.0)).h(px(64.0)).rounded_lg().overflow_hidden()
        .bg(t.surface).flex().items_center().justify_center()
        .child(match cover {
            Some(c) => img(c.clone()).size_full().object_fit(ObjectFit::Cover).into_any_element(),
            None => div().text_size(px(24.0)).text_color(t.muted_fg).child("♪").into_any_element(),
        });

    // 细进度条：外轨 + 按比例撑开的填充，高度 3px 不抢视觉
    let progress = div().w_full().h(px(3.0)).rounded_full().bg(t.border).overflow_hidden()
        .child(div().h(px(3.0)).rounded_full().bg(t.accent_light)
            .w(relative(pct)));

    div().id("mini-player").size_full()
        .flex().items_center().gap_3().px_4()
        .bg(t.surface)
        .child(art)
        // 中间：信息 + 进度
        .child(
            div().flex_1().min_w(px(0.0)).flex_col().gap_1()
                .child(
                    div().flex().items_center().justify_between().gap_2()
                        .child(div().flex_1().min_w(px(0.0)).text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM).text_color(t.fg)
                            .text_ellipsis().child(title.to_string()))
                        .child(div().flex_none().text_xs().text_color(t.muted_fg)
                            .child(format!("{} / {}", crate::audio::fmt_time(cur_t), crate::audio::fmt_time(tot_t))))
                )
                .child(div().text_xs().text_color(t.muted_fg).text_ellipsis().child(artist.to_string()))
                .child(progress)
        )
        // 右侧控制：上一曲 / 播放暂停 / 下一曲 / 还原窗口
        .child(
            div().flex_none().flex().items_center().gap_2()
                .child(
                    div().id("m-prev").w(px(28.0)).h(px(28.0)).rounded_full()
                        .flex().items_center().justify_center().cursor_pointer()
                        .hover(|s| s.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                        .on_click(cx.listener(crate::MusicPlayer::prev))
                        .child(svg().path("icons/prev.svg").size_4().text_color(t.muted_fg))
                )
                .child(
                    div().id("m-play").w(px(40.0)).h(px(40.0)).rounded_full()
                        .bg(t.play_btn).flex().items_center().justify_center().cursor_pointer()
                        .shadow_lg()
                        .hover(|s| s.opacity(0.9))
                        .on_click(cx.listener(crate::MusicPlayer::play_pause))
                        .child(svg().path(if playing { "icons/pause.svg" } else { "icons/play.svg" })
                            .size_5().text_color(t.play_btn_fg))
                )
                .child(
                    div().id("m-next").w(px(28.0)).h(px(28.0)).rounded_full()
                        .flex().items_center().justify_center().cursor_pointer()
                        .hover(|s| s.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                        .on_click(cx.listener(crate::MusicPlayer::next))
                        .child(svg().path("icons/next.svg").size_4().text_color(t.muted_fg))
                )
                // 退出迷你模式（还原窗口尺寸）
                .child(
                    div().id("m-restore").w(px(28.0)).h(px(28.0)).rounded_full()
                        .flex().items_center().justify_center().cursor_pointer()
                        .hover(|s| s.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                        .on_click(cx.listener(crate::MusicPlayer::toggle_mini_mode))
                        .child(svg().path("icons/restore.svg").size_3_5().text_color(t.accent_light))
                )
        )
        .into_any()
}
