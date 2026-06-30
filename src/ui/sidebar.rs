// ─── 侧边栏模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::ActiveTheme as _;
use crate::models::Folder;

/// 构建侧边栏
pub fn build_sidebar(
    folders: &[Folder],
    current: &Option<(usize, usize)>,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    let mut folder_els: Vec<AnyElement> = Vec::new();

    for fi in 0..folders.len() {
        let name = folders[fi].name.clone();
        let exp = folders[fi].expanded;
        let cnt = folders[fi].tracks.len();
        let mut track_els: Vec<AnyElement> = Vec::new();

        // 展开时显示曲目列表
        if exp {
            for ti in 0..cnt {
                let t = folders[fi].tracks[ti].clone();
                let cur = *current == Some((fi, ti));
                let dur = crate::audio::fmt_time(t.duration);
                track_els.push(
                    div().id(("t", (fi * 10000 + ti) as u64))
                        .flex().items_center().justify_between()
                        .px_3().py_2().rounded_md()
                        .bg(if cur { cx.theme().list_active } else { cx.theme().background })
                        .hover(|s| s.bg(cx.theme().accent))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, e, w, cx| this.play_at(fi, ti, e, w, cx)))
                        .child(div().flex_col().overflow_hidden().flex_1()
                            .child(div().text_color(if cur { cx.theme().primary } else { cx.theme().foreground })
                                .text_sm().text_ellipsis().whitespace_nowrap().child(t.title))
                            .child(div().text_color(cx.theme().muted_foreground).text_xs().text_ellipsis().whitespace_nowrap().child(t.artist)))
                        .child(div().text_color(cx.theme().muted_foreground).text_xs().flex_none().ml_2().child(dur))
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
                        .hover(|s| s.bg(cx.theme().accent))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, e, w, cx| this.toggle_folder(fidx, e, w, cx)))
                        .child(div().flex().items_center().gap_2()
                            .child(div().text_color(cx.theme().primary).text_xs().child(if exp { "▼" } else { "▶" }))
                            .child(div().text_color(cx.theme().foreground).text_sm().child(name)))
                        .child(div().text_color(cx.theme().muted_foreground).text_xs().child(format!("{}", cnt))))
                .when(exp, |this| this.child(div().flex_col().ml_3().gap_0p5().my_1().children(track_els)))
                .into_any()
        );
    }

    let has_folders = !folders.is_empty();

    div().id("sidebar").flex_none().w(px(260.0)).flex_col()
        .bg(cx.theme().list).border_r_1().border_color(cx.theme().border)
        // 标题栏
        .child(div().flex().items_center().justify_between().px_3().py_1()
            .border_b_1().border_color(cx.theme().border)
            .child(div().text_sm().font_weight(gpui::FontWeight::BOLD).text_color(cx.theme().foreground).child("音乐库"))
            .child(div().id("hdr-add").text_color(cx.theme().muted_foreground).text_lg().cursor_pointer()
                .hover(|s| s.text_color(cx.theme().foreground))
                .on_click(cx.listener(crate::MusicPlayer::add_folder)).child("+")))
        // 曲目列表
        .child(div().id("sidebar-list").flex_1().flex_col().id("sidebar-scroll").overflow_y_scroll()
            .child(div().flex_col().p_2().gap_1()
                .when(!has_folders, |this| this.child(div().text_color(cx.theme().muted_foreground).text_center().py_12().text_sm().child("点击 + 添加音乐文件夹")))
                .children(folder_els)))
        .into_any()
}
