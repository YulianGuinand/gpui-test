//! GPUI Kit messaging demo for Android.
//!
//! The Android host is a `NativeActivity` (`dev.gpui.mobile.GpuiActivity`)
//! provided by the `gpui-mobile` compatibility fork: the activity dlopens the
//! `libmy_gpui_mobile.so` produced by this crate and calls [`android_main`].
//!
//! Modules:
//! - [`mock`]  — deterministic conversation/message generator (std only)
//! - [`perf`]  — FPS / build-time / virtual-row counters and the HUD overlay
//! - [`app`]   — root view: conversation list ⇄ chat navigation
//! - [`screens::chat`] — virtualized conversation screen with composer

pub mod mock;
pub mod perf;

#[cfg(any(target_os = "android", target_os = "ios"))]
pub mod app;
#[cfg(any(target_os = "android", target_os = "ios"))]
pub mod screens;

// ── Android entry point ─────────────────────────────────────────────────────

#[cfg(target_os = "android")]
use gpui_mobile::android::jni;

/// Called by the `android-activity` crate on a dedicated native thread.
/// Blocks until the Android event loop exits.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("gpui-msg"),
    );

    jni::install_panic_hook();
    log::info!("android_main: entered");

    let _platform = jni::init_platform(&app);
    let shared = match jni::shared_platform() {
        Some(s) => s,
        None => {
            log::error!("android_main: shared_platform() returned None — aborting");
            return;
        }
    };

    log::info!("android_main: creating GPUI Application");
    gpui::Application::with_platform(shared.into_rc()).run(|cx: &mut gpui::App| {
        open_main_window(cx);
    });
    log::info!("android_main: Application::run returned");
}

// ── Shared window creation ──────────────────────────────────────────────────

/// Open the fullscreen window hosting the GPUI Kit root view.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn open_main_window(cx: &mut gpui::App) {
    use gpui::AppContext as _;

    gpui_kit::init(cx);
    gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);

    let result = cx.open_window(gpui::WindowOptions::default(), |window, cx| {
        let view = cx.new(|cx| app::AppView::new(window, cx));
        cx.new(|cx| gpui_kit::component::Root::new(view, window, cx))
    });

    match result {
        Ok(_) => log::info!("open_main_window: ok"),
        Err(e) => log::error!("open_main_window: failed: {e:#}"),
    }
    cx.activate(true);
}
