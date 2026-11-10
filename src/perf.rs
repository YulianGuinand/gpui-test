//! Lightweight performance HUD (fps / frame build time / virtual rows built).
//!
//! `gpui-fps` does not support Android, so the demo carries its own counters:
//! the root view reports one "frame" per render pass, and the row-render
//! closures of `MessageScroller` report how many rows were actually built
//! (virtualization proof).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Shared with every `MessageScroller` row closure: counts rows built during
/// the current render pass.
pub type RowsCounter = Rc<Cell<usize>>;

#[derive(Clone, Debug)]
pub struct FrameStats {
    /// Renders per second over the last reporting window (2 s).
    pub fps: f64,
    /// Time spent building the element tree in the last render pass, in ms.
    pub build_ms: f64,
    /// Rows built during the last render pass.
    pub rows: usize,
}

struct Inner {
    frames: usize,
    window_start: Option<Instant>,
    last_build_ms: f64,
    frames_since_report: usize,
    last_stats: Option<FrameStats>,
}

#[derive(Clone)]
pub struct Perf {
    inner: Rc<RefCell<Inner>>,
    counter: RowsCounter,
    /// Extra label appended to logs (current screen).
    label: Rc<RefCell<String>>,
    /// Whether the HUD overlay is drawn (counting continues either way).
    visible: Rc<Cell<bool>>,
}

impl Default for Perf {
    fn default() -> Self {
        Self::new()
    }
}

impl Perf {
    pub fn new() -> Self {
        Perf {
            inner: Rc::new(RefCell::new(Inner {
                frames: 0,
                window_start: None,
                last_build_ms: 0.0,
                frames_since_report: 0,
                last_stats: None,
            })),
            counter: Rc::new(Cell::new(0)),
            label: Rc::new(RefCell::new(String::from("boot"))),
            visible: Rc::new(Cell::new(true)),
        }
    }

    /// Handle given to message-row closures.
    pub fn rows_counter(&self) -> RowsCounter {
        self.counter.clone()
    }

    pub fn set_label(&self, label: impl Into<String>) {
        *self.label.borrow_mut() = label.into();
    }

    pub fn toggle_visible(&self) -> bool {
        let v = !self.visible.get();
        self.visible.set(v);
        v
    }

    /// Called at the start of a render pass; returns the build timestamp.
    pub fn begin_frame(&self) -> Instant {
        self.inner.borrow_mut().last_build_ms = 0.0;
        Instant::now()
    }

    /// Called once the element tree is built, with the timestamp from
    /// [`begin_frame`]. Returns the stats to display (some every 2 s, used
    /// both for the HUD and for `logcat`).
    pub fn end_frame(&self, build_started: Instant) -> Option<FrameStats> {
        let build_ms = build_started.elapsed().as_secs_f64() * 1000.0;
        let rows = self.counter.replace(0);
        let now = Instant::now();
        let mut inner = self.inner.borrow_mut();

        inner.last_build_ms = build_ms;
        inner.window_start.get_or_insert(now);

        let elapsed = now.duration_since(inner.window_start.unwrap());
        if elapsed >= Duration::from_secs(2) {
            let fps = (inner.frames_since_report + 1) as f64 / elapsed.as_secs_f64();
            let stats = FrameStats {
                fps,
                build_ms,
                rows,
            };
            log::info!(
                "perf [{}] fps={fps:.1} build={build_ms:.2}ms rows={} window={}ms",
                self.label.borrow(),
                stats.rows,
                elapsed.as_millis()
            );
            inner.window_start = Some(now);
            inner.frames_since_report = 0;
            inner.last_stats = Some(stats.clone());
        } else {
            inner.frames_since_report += 1;
        }

        inner.frames += 1;
        inner.last_stats.clone()
    }

    pub fn hud_text(&self) -> Option<String> {
        if !self.visible.get() {
            return None;
        }
        let inner = self.inner.borrow();
        let stats = inner.last_stats.as_ref()?;
        Some(format!(
            "{} · {:.0} fps · {:.1} ms · {} rows",
            self.label.borrow(),
            stats.fps,
            stats.build_ms,
            stats.rows
        ))
    }

    #[allow(dead_code)]
    pub fn frame_count(&self) -> usize {
        self.inner.borrow().frames
    }
}
