//! Root application view: conversation list ⇄ chat navigation, dataset
//! switching, and the performance HUD overlay.

use std::time::Instant;

use crate::mock::{self, Database, Scale};
use crate::perf::Perf;
use crate::screens::chat::ChatState;

use gpui::{InteractiveElement, StatefulInteractiveElement, px, rgb};

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    AnyElement, Context, Div, IntoElement, ParentElement as _, Render, Styled as _, Window, div,
};

// ── Navigation ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    List,
    Chat(usize),
}

// ── Root view ───────────────────────────────────────────────────────────────

pub struct AppView {
    pub db: Database,
    pub screen: Screen,
    pub chat: Option<ChatState>,
    pub perf: Perf,
}

impl AppView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let start = Instant::now();
        let db = Database::generate(Scale::Large);
        log::info!(
            "dataset {} — {} messages in {} conversations, generated in {} ms (measured {} ms)",
            db.scale,
            db.message_count,
            db.conversations.len(),
            start.elapsed().as_millis(),
            db.generation_ms
        );

        let perf = Perf::new();
        perf.set_label("list");

        Self {
            db,
            screen: Screen::List,
            chat: None,
            perf,
        }
    }

    // ── Actions ─────────────────────────────────────────────────────────

    pub fn open_chat(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.db.conversations.get(index).is_none() {
            return;
        }
        if let Some(conv) = self.db.conversations.get_mut(index) {
            conv.unread = 0;
        }
        let now = self.db.now;
        let rows = self.perf.rows_counter();
        let chat = ChatState::new(index, &self.db.conversations[index], now, rows, window, cx);

        self.screen = Screen::Chat(index);
        self.chat = Some(chat);
        self.perf.set_label(format!("chat {}", index + 1));
        cx.notify();
    }

    pub fn switch_scale(&mut self, scale: Scale, cx: &mut Context<Self>) {
        if self.db.scale == scale {
            return;
        }
        self.screen = Screen::List;
        self.chat = None;
        self.perf.set_label("list");

        log::info!("regenerating dataset at scale {scale}…");
        let start = Instant::now();
        self.db = Database::generate(scale);
        log::info!(
            "regenerated {} messages in {} ms",
            self.db.message_count,
            start.elapsed().as_millis()
        );
        cx.notify();
    }

    pub fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(chat) = self.chat.as_mut() {
            chat.send(window, cx);
        }
    }

    // ── Conversation list ───────────────────────────────────────────────

    fn render_list(&mut self, cx: &mut Context<Self>) -> Div {
        let conv_count = self.db.conversations.len();
        let total = self.db.total_messages();
        let unread = self.db.total_unread();
        let gen_ms = self.db.generation_ms;
        let scale = self.db.scale;

        let rows: Vec<AnyElement> = (0..conv_count)
            .map(|index| self.conversation_row(index, cx))
            .collect();

        let scale_buttons = h_flex().gap_1().children(Scale::ALL.into_iter().map(|s| {
            let button = Button::new(scale_id(s)).xsmall().label(s.label());
            let button = if s == scale {
                button.primary()
            } else {
                button.outline()
            };
            button.on_click(cx.listener(move |this, _, _, cx| this.switch_scale(s, cx)))
        }));

        let header = h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_3()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_1()
                    .min_w_0()
                    .child(div().text_lg().font_semibold().child("Messages"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{conv_count} conversations · {total} messages · {unread} non lus · gen {gen_ms} ms"
                            )),
                    ),
            )
            .child(scale_buttons);

        v_flex()
            .size_full()
            .child(header)
            .child(
                div()
                    .id("conv-list")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .px_2()
                    .pb_3()
                    .children(rows),
            )
    }

    fn conversation_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let conv = &self.db.conversations[index];
        let now = self.db.now;

        let title = conv.title.clone();
        let preview = mock::truncate_chars(conv.preview(), 52);
        let stamp = mock::format_list_stamp(conv.last_at(), now);
        let unread = conv.unread;
        let theme = cx.theme();

        let badge: AnyElement = if unread > 0 {
            div()
                .px_2()
                .py_1()
                .rounded(theme.radius_full())
                .bg(theme.primary)
                .text_color(theme.primary_foreground)
                .text_xs()
                .child(format!("{unread}"))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(("conv", index))
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_chat(index, window, cx)
            }))
            .child(Avatar::new().name(title.clone()).size_8())
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(div().flex_1().min_w_0().font_semibold().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(stamp),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(preview),
                    ),
            )
            .child(badge)
            .into_any_element()
    }
}

fn scale_id(scale: Scale) -> &'static str {
    match scale {
        Scale::Small => "scale-10k",
        Scale::Large => "scale-50k",
        Scale::Huge => "scale-100k",
    }
}

// ── HUD overlay ─────────────────────────────────────────────────────────────

fn hud_bar(text: String) -> Div {
    h_flex()
        .w_full()
        .justify_end()
        .px_2()
        .py_1()
        .bg(rgb(0x101018))
        .text_color(rgb(0x7CFC9A))
        .text_xs()
        .child(text)
}

// ── Render ──────────────────────────────────────────────────────────────────

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let build_start = self.perf.begin_frame();

        // Back navigation requested by the chat header.
        if self.chat.as_ref().is_some_and(|c| c.want_back) {
            self.chat = None;
            self.screen = Screen::List;
            self.perf.set_label("list");
            log::info!("back to list");
        }

        let content: AnyElement = match self.screen {
            Screen::List => self.render_list(cx).into_any_element(),
            Screen::Chat(_) => match self.chat.as_mut() {
                Some(chat) => chat.render(window, cx).into_any_element(),
                None => div().into_any_element(),
            },
        };

        self.perf.end_frame(build_start);
        let hud = self.perf.hud_text().map(hud_bar);

        // Clear the Android status bar (top) and gesture bar (bottom):
        // gpui-mobile reports safe_area_insets = 0 in fullscreen, so the
        // layout reserves the space itself.
        let mut root = div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(32.))
            .pb(px(24.));
        if let Some(hud) = hud {
            root = root.child(hud);
        }
        root.child(content)
    }
}
