use crate::{
    nes::{Running, renderer::Renderer},
    platform::{BuilderExt, Initialize},
};
use std::path::{Path, PathBuf};
use winit::window::{Fullscreen, WindowAttributes};

/// Hide the Android navigation bar and status bar using immersive sticky mode.
///
/// Uses `View.setSystemUiVisibility` with `SYSTEM_UI_FLAG_IMMERSIVE_STICKY`,
/// `SYSTEM_UI_FLAG_HIDE_NAVIGATION`, and `SYSTEM_UI_FLAG_FULLSCREEN`.
/// Works on API 26+ (deprecated but still functional on API 30+).
pub fn hide_navigation_bar_impl() {
    use jni::{
        JavaVM,
        objects::{JObject, JValue},
    };

    // SYSTEM_UI_FLAG_HIDE_NAVIGATION | SYSTEM_UI_FLAG_FULLSCREEN | SYSTEM_UI_FLAG_IMMERSIVE_STICKY
    const FLAGS: i32 = 0x0000_0002 | 0x0000_0004 | 0x0000_1000;

    let ctx = ndk_context::android_context();
    let vm = match unsafe { JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to get JavaVM: {e:?}");
            return;
        }
    };
    let mut env = match vm.attach_current_thread_permanently() {
        Ok(env) => env,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to attach JNI thread: {e:?}");
            return;
        }
    };

    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let window = match env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .and_then(|v| v.l())
    {
        Ok(w) => w,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to get window: {e:?}");
            return;
        }
    };

    let decor_view = match env
        .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
        .and_then(|v| v.l())
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to get decor view: {e:?}");
            return;
        }
    };

    if let Err(e) = env.call_method(
        &decor_view,
        "setSystemUiVisibility",
        "(I)V",
        &[JValue::Int(FLAGS)],
    ) {
        log::warn!("hide_navigation_bar: setSystemUiVisibility failed: {e:?}");
    }
}

/// Method for platforms supporting opening a file dialog.
pub fn open_file_dialog_impl(
    _title: impl Into<String>,
    _name: impl Into<String>,
    _extensions: &[impl ToString],
    _dir: Option<impl AsRef<Path>>,
) -> anyhow::Result<Option<PathBuf>> {
    // No file dialogs on Android
    Ok(None)
}

/// Speak the given text out loud.
pub const fn speak_text_impl(_text: &str) {}

impl Initialize for Running {
    fn initialize(&mut self) -> anyhow::Result<()> {
        hide_navigation_bar_impl();
        Ok(())
    }
}

impl Initialize for Renderer {
    fn initialize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl BuilderExt for WindowAttributes {
    fn with_platform(self, _title: &str) -> Self {
        // Android has no window chrome; force borderless fullscreen so the app
        // fills the entire display from launch.
        self.with_fullscreen(Some(Fullscreen::Borderless(None)))
    }
}

pub mod renderer {
    use super::*;
    use crate::nes::{config::Config, event::Response};

    pub fn constrain_window_to_viewport_impl(
        _renderer: &Renderer,
        _desired_window_width: f32,
        _cfg: &Config,
    ) -> Response {
        Response::default()
    }
}
