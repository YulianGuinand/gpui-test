//! Chat screen: virtualized `MessageScroller` of `Message` rows + composer.
//!
//! The heavy conversation data lives in an `Arc<Vec<Message>>` (zero-copy
//! when opening a thread); messages the user appends — and the streamed
//! simulated replies — go to a small `extra` buffer.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::app::AppView;
use crate::mock::{self, Conversation, Delivery, Message as ChatMessage};
use crate::perf::RowsCounter;

use gpui::ElementId;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    avatar::Avatar,
    bubble::{Bubble, BubbleVariant},
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::{Input, InputState},
    marker::{Marker, MarkerContent, MarkerVariant},
    message::{
        Message as MessageRow, MessageAlignment, MessageContent, MessageFooter, MessageHeader,
    },
    message_scroller::{MessageScroller, MessageScrollerState},
};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Div, Entity, IntoElement, ParentElement as _,
    Styled as _, StyleRefinement, Task, Window, div,
};

const DAY: u64 = 86_400;

// ── Data shared with the row renderer closure ───────────────────────────────

pub struct ChatStore {
    base: Arc<Vec<ChatMessage>>,
    extra: RefCell<Vec<ChatMessage>>,
    senders: Arc<Vec<String>>,
    now: u64,
}

impl ChatStore {
    pub fn len(&self) -> usize {
        self.base.len() + self.extra.borrow().len()
    }

    pub fn get(&self, index: usize) -> Option<ChatMessage> {
        if index < self.base.len() {
            Some(self.base[index].clone())
        } else {
            self.extra.borrow().get(index - self.base.len()).cloned()
        }
    }

    fn push(&self, message: ChatMessage) {
        self.extra.borrow_mut().push(message);
    }

    fn append_to_last_body(&self, chunk: &str) {
        let mut extra = self.extra.borrow_mut();
        if let Some(message) = extra.last_mut() {
            if !message.body.is_empty() {
                message.body.push(' ');
            }
            message.body.push_str(chunk);
        }
    }
}

// ── Screen state (owned by AppView) ─────────────────────────────────────────

pub struct ChatState {
    pub conv_index: usize,
    pub title: String,
    pub store: Rc<ChatStore>,
    pub scroller: Entity<MessageScrollerState>,
    pub composer: Entity<InputState>,
    pub rows: RowsCounter,
    /// Set by the back button, consumed by AppView on the next render.
    pub want_back: bool,
    /// Index that carries the "Non lu" separator, if any.
    pub unread_at: Option<usize>,
    pub streaming: bool,
    stream_task: Option<Task<()>>,
    next_id: u64,
}

impl ChatState {
    pub fn new(
        conv_index: usize,
        conversation: &Conversation,
        now: u64,
        rows: RowsCounter,
        window: &mut Window,
        cx: &mut Context<AppView>,
    ) -> Self {
        let len = conversation.messages.len();
        let store = Rc::new(ChatStore {
            base: Arc::clone(&conversation.messages),
            extra: RefCell::new(Vec::new()),
            senders: Arc::new(conversation.senders.clone()),
            now,
        });

        let scroller = cx.new(|cx| MessageScrollerState::new(len, cx));
        cx.observe(&scroller, |_, _, cx| cx.notify()).detach();

        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("Write a message…"));

        let unread_at = if conversation.unread > 0 && conversation.unread < len {
            Some(len - conversation.unread)
        } else {
            None
        };

        log::info!(
            "open chat #{} \"{}\" — {} messages, unread_at={unread_at:?}",
            conversation.id,
            conversation.title,
            len
        );

        Self {
            conv_index,
            title: conversation.title.clone(),
            store,
            scroller,
            composer,
            rows,
            want_back: false,
            unread_at,
            streaming: false,
            stream_task: None,
            next_id: 9_000_000_000,
        }
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&mut self, _window: &mut Window, cx: &mut Context<AppView>) -> Div {
        let store = self.store.clone();
        let rows = self.rows.clone();
        let unread_at = self.unread_at;
        let len = store.len();
        let title = self.title.clone();
        let streaming = self.streaming;
        let theme_bg = cx.theme().background;

        let header = h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("chat-back")
                    .ghost()
                    .xsmall()
                    .label("‹ Retours")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(chat) = this.chat.as_mut() {
                            chat.want_back = true;
                        }
                        cx.notify();
                    })),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(div().font_semibold().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{len} messages · streaming: {}",
                                if streaming { "on" } else { "off" }
                            )),
                    ),
            )
            .child(
                Button::new("chat-hud")
                    .ghost()
                    .xsmall()
                    .label("HUD")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.perf.toggle_visible();
                        cx.notify();
                    })),
            );

        let list = MessageScroller::new(
            "chat-rows",
            self.scroller.clone(),
            move |index, _window, cx| row_element(index, &store, &rows, unread_at, cx),
        )
        .with_list_style(StyleRefinement::default().px_4().py_3())
        .with_bottom_fade(theme_bg);

        let composer = div()
            .w_full()
            .px_3()
            .pt_2()
            .pb_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded_xl()
                    .bg(cx.theme().muted)
                    .child(Input::new(&self.composer).appearance(false))
                    .child(
                        h_flex().w_full().justify_end().child(
                            Button::new("chat-send")
                                .primary()
                                .xsmall()
                                .label("Envoyer")
                                .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
                        ),
                    ),
            );

        v_flex()
            .size_full()
            .child(header)
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(list),
            )
            .child(composer)
    }

    // ── Sending + simulated streamed reply ───────────────────────────────

    pub fn send(&mut self, window: &mut Window, cx: &mut Context<AppView>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        if text.is_empty() || self.streaming {
            return;
        }

        self.composer
            .update(cx, |input, cx| input.set_value("", window, cx));

        let at = mock::now_secs();
        let id = self.next_id;
        self.next_id += 1;
        self.store.push(ChatMessage {
            id,
            sender: 0,
            body: text,
            at,
            delivery: Some(Delivery::Sent),
        });
        let reply_index = self.store.len();
        let appended = self.store.extra.borrow().len();
        self.scroller.update(cx, |state, cx| {
            let _ = state.append(1, cx);
        });
        cx.notify();

        log::info!(
            "send: total={} extra={appended}",
            self.store.len()
        );

        self.spawn_reply(reply_index, cx);
    }

    fn spawn_reply(&mut self, _reply_index: usize, cx: &mut Context<AppView>) {
        let store = self.store.clone();
        let seed = mock::now_secs() ^ (store.len() as u64).wrapping_mul(0x9E37_79B9);
        let reply = mock::auto_reply(seed);
        let words: Vec<String> = reply.split_whitespace().map(str::to_string).collect();

        self.streaming = true;

        self.stream_task = Some(cx.spawn(async move |this, cx| {
            // Thinking pause.
            cx.background_executor()
                .timer(Duration::from_millis(450))
                .await;

            // Push the (empty) reply row.
            let started = this
                .update(cx, |this, cx| {
                    let Some(chat) = this.chat.as_mut() else {
                        return false;
                    };
                    if !Rc::ptr_eq(&chat.store, &store) {
                        return false;
                    }
                    store.push(ChatMessage {
                        id: 0,
                        sender: 1,
                        body: String::new(),
                        at: mock::now_secs(),
                        delivery: None,
                    });
                    chat.scroller.update(cx, |state, cx| {
                        let _ = state.append(1, cx);
                    });
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !started {
                return;
            }

            for word in &words {
                cx.background_executor()
                    .timer(Duration::from_millis(85))
                    .await;

                let cont = this
                    .update(cx, |this, cx| {
                        let Some(chat) = this.chat.as_mut() else {
                            return false;
                        };
                        if !Rc::ptr_eq(&chat.store, &store) {
                            return false;
                        }
                        store.append_to_last_body(word);
                        let len = store.len();
                        chat.scroller.update(cx, |state, cx| {
                            let _ = state.remeasure_items(len - 1..len, cx);
                        });
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !cont {
                    return;
                }
            }

            let _ = this.update(cx, |this, cx| {
                if let Some(chat) = this.chat.as_mut() {
                    chat.streaming = false;
                }
                cx.notify();
            });
            log::info!("reply streamed ({} words)", words.len());
        }));
    }
}

// ── One virtual row ─────────────────────────────────────────────────────────

fn row_element(
    index: usize,
    store: &Rc<ChatStore>,
    rows: &RowsCounter,
    unread_at: Option<usize>,
    cx: &App,
) -> AnyElement {
    rows.set(rows.get() + 1);

    let Some(message) = store.get(index) else {
        return div().into_any_element();
    };
    let prev = if index > 0 {
        store.get(index - 1)
    } else {
        None
    };
    let next = store.get(index + 1);

    let theme = cx.theme();
    let new_day = prev
        .as_ref()
        .is_none_or(|p| p.at / DAY != message.at / DAY);
    let same_sender = prev.as_ref().is_some_and(|p| p.sender == message.sender);
    let continues_group = same_sender && !new_day;
    let last_of_group = next
        .as_ref()
        .is_none_or(|n| n.sender != message.sender || n.at / DAY != message.at / DAY);

    let sender_name = store
        .senders
        .get(message.sender)
        .cloned()
        .unwrap_or_else(|| "?".into());

    let avatar: AnyElement = if continues_group {
        div()
            .w(gpui::px(32.))
            .h(gpui::px(32.))
            .into_any_element()
    } else {
        Avatar::new().name(sender_name.clone()).size_8().into_any_element()
    };

    let bubble = if message.is_outgoing() {
        Bubble::new().child(message.body.clone())
    } else {
        Bubble::new()
            .with_variant(BubbleVariant::Secondary)
            .child(message.body.clone())
    };

    let mut row = MessageRow::new()
        .id(ElementId::NamedInteger("row".into(), index as u64))
        .alignment(if message.is_outgoing() {
            MessageAlignment::End
        } else {
            MessageAlignment::Start
        })
        .avatar(avatar)
        .content(MessageContent::new().bubble(bubble));

    if !continues_group {
        row = row.header(
            MessageHeader::new()
                .child(sender_name)
                .child("·")
                .child(mock::format_time(message.at)),
        );
    }

    if last_of_group {
        if let Some(delivery) = message.delivery {
            row = row.footer(MessageFooter::new().child(delivery.label()));
        }
    }

    let mut stack = v_flex().w_full().min_w_0().gap_2();

    if new_day {
        let label = mock::format_day(message.at, store.now);
        stack = stack.child(
            div()
                .w_full()
                .flex()
                .justify_center()
                .py_2()
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded(theme.radius_full())
                        .bg(theme.muted)
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(label),
                ),
        );
    }

    if unread_at == Some(index) {
        stack = stack.child(
            Marker::new()
                .with_variant(MarkerVariant::Separator)
                .content(MarkerContent::new().child("Non lu")),
        );
    }

    stack.child(row).into_any_element()
}
