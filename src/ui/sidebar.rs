// ─── 侧边栏模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use crate::theme::ThemeConfig;
use crate::models::Folder;

pub fn build_sidebar(
    folders: &[Folder],
    current: &Option<(usize, usize)>,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    let mut folder_els: Vec<AnyElement> = Vec::new();
    for fi in 0..folders.len() {
        let name = folders[fi].name.clone();
        let exp = folders[fi].expanded;
        let cnt = folders[fi].tracks.len();
        let mut track_els: Vec<AnyElement> = Vec::new();
        if exp {
            for ti in 0..cnt {
                let track = folders[fi].tracks[ti].clone();
                let cur = *current == Some((fi, ti));
                let dur = crate::audio::fmt_time(track.duration);
                track_els.push(
                    div().id(("t", (fi * 10000 + ti) as u64))
                        .flex().items_center().gap_3()
                        .px_3().py_2().rounded_md()
                        .when(cur, |this| this.bg(t.active))
                        .hover(|style| style.bg(t.hover))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, e, w, cx| this.play_at(fi, ti, e, w, cx)))
                        // 序号
                        .child(div().flex_none().w(px(24.0)).text_xs().text_color(t.muted_fg)
                            .text_center().child(format!("{}", ti + 1)))
                        // 封面占位
                        .child(div().flex_none().w(px(40.0)).h(px(40.0)).rounded_md()
                            .bg(t.surface).flex().items_center().justify_center()
                            .text_xs().text_color(t.muted_fg).child("♪"))
                        // 歌曲信息
                        .child(div().flex_1().flex_col().overflow_hidden()
                            .child(div().text_sm().text_ellipsis().whitespace_nowrap()
                                .text_color(if cur { t.accent_light } else { t.fg })
                                .child(track.title))
                            .child(div().text_xs().text_ellipsis().whitespace_nowrap()
                                .text_color(t.muted_fg).child(track.artist)))
                        // 时长
                        .child(div().flex_none().text_xs().text_color(t.muted_fg).child(dur))
                        .into_any()
                );
            }
        }
        let fidx = fi;
        folder_els.push(
            div().id(("f", fidx as u64)).flex_col()
                .child(
                    div().id(("fh", fidx as u64)).flex().items_center().justify_between()
                        .px_3().py_2().rounded_md()
                        .hover(|style| style.bg(t.hover))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, e, w, cx| this.toggle_folder(fidx, e, w, cx)))
                        .child(div().flex().items_center().gap_2()
                            .child(div().text_color(t.accent_light).text_xs().child(if exp { "▼" } else { "▶" }))
                            .child(div().text_color(t.fg).text_sm().font_weight(gpui::FontWeight::MEDIUM).child(name)))
                        .child(div().text_color(t.muted_fg).text_xs().child(format!("{}", cnt))))
                .when(exp, |this| this.child(div().flex_col().ml_2().gap_0p5().my_1().children(track_els)))
                .into_any()
        );
    }
    let has_folders = !folders.is_empty();

    // 搜索栏
    let search_bar = div().flex().items_center().gap_2()
        .px_3().py_2().rounded_lg().mt_3()
        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
        .border_1().border_color(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 })
        .child(svg().path("icons/search.svg").size_4().text_color(t.muted_fg))
        .child(div().text_sm().text_color(t.muted_fg).child("搜索歌曲、歌手..."));

    // 标签页
    let tabs = div().flex().gap_2().px_1()
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .bg(Hsla { h: 252.0, s: 0.73, l: 0.64, a: 0.15 })
                .text_color(t.fg).child("全部")
        )
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.muted_fg).child("播放列表")
        )
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.muted_fg).child("专辑")
        );

    div().id("sidebar").flex_none().w(px(280.0)).flex_col()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.5 })
        .border_r_1().border_color(t.border)
        // 标题栏
        .child(
            div().px_4().pt_5().pb_4().flex_col().gap_5()
                .child(div().flex().items_center().gap_2()
                    .child(svg().path("icons/music.svg").size_5().text_color(t.accent_light))
                    .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).text_color(t.fg).child("音乐库")))
                .child(search_bar)
        )
        // 标签页
        .child(div().px_3().pb_2().child(tabs))
        // 歌曲列表
        .child(div().id("sidebar-list").flex_1().flex_col().overflow_hidden()
            .px_3().pb_3()
            .child(div().flex_col().gap_0p5()
                .when(!has_folders, |this| this.child(
                    div().text_color(t.muted_fg).text_center().py_12().text_sm()
                        .child("在设置中添加音乐文件夹")))
                .children(folder_els)))
        .into_any()
}
