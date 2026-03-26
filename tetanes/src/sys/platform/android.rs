use crate::{
    nes::{Running, renderer::Renderer},
    platform::{BuilderExt, Initialize},
};
use std::path::{Path, PathBuf};
use winit::window::{Fullscreen, WindowAttributes};

/// Hide the Android navigation bar and status bar using immersive mode.
///
/// Uses `WindowInsetsController` on API 30+ for reliable behavior,
/// with fallback to `View.setSystemUiVisibility` on older devices.
pub fn hide_navigation_bar_impl() {
    use jni::{
        JavaVM,
        objects::{JObject, JValue},
    };

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

    let sdk_int = get_sdk_int(&mut env);

    if sdk_int >= 30 {
        // Use WindowInsetsController (API 30+) — the modern, reliable approach.
        let controller = match env
            .call_method(
                &window,
                "getInsetsController",
                "()Landroid/view/WindowInsetsController;",
                &[],
            )
            .and_then(|v| v.l())
        {
            Ok(c) if !c.is_null() => c,
            Ok(_) => {
                log::warn!("hide_navigation_bar: WindowInsetsController is null");
                return;
            }
            Err(e) => {
                log::warn!("hide_navigation_bar: getInsetsController failed: {e:?}");
                return;
            }
        };

        // WindowInsets.Type.systemBars() — covers both status bar and navigation bar.
        let system_bars = match env
            .call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])
            .and_then(|v| v.i())
        {
            Ok(v) => v,
            Err(e) => {
                log::warn!("hide_navigation_bar: systemBars() failed: {e:?}");
                return;
            }
        };

        if let Err(e) =
            env.call_method(&controller, "hide", "(I)V", &[JValue::Int(system_bars)])
        {
            log::warn!("hide_navigation_bar: hide() failed: {e:?}");
        }

        // BEHAVIOR_SHOW_TRANSIENT_BARS_BY_GESTURE = 2
        // Bars temporarily appear on swipe but auto-hide.
        if let Err(e) =
            env.call_method(&controller, "setSystemBarsBehavior", "(I)V", &[JValue::Int(2)])
        {
            log::warn!("hide_navigation_bar: setSystemBarsBehavior failed: {e:?}");
        }
    } else {
        // Fallback: setSystemUiVisibility (API 24–29).
        // SYSTEM_UI_FLAG_HIDE_NAVIGATION | SYSTEM_UI_FLAG_FULLSCREEN | SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        const FLAGS: i32 = 0x0000_0002 | 0x0000_0004 | 0x0000_1000;

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
}

/// One-time Android display setup: force landscape and configure display cutout mode.
pub fn setup_android_display() {
    use jni::{
        JavaVM,
        objects::{JObject, JValue},
    };

    let ctx = ndk_context::android_context();
    let vm = match unsafe { JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(e) => {
            log::warn!("setup_android_display: failed to get JavaVM: {e:?}");
            return;
        }
    };
    let mut env = match vm.attach_current_thread_permanently() {
        Ok(env) => env,
        Err(e) => {
            log::warn!("setup_android_display: failed to attach JNI thread: {e:?}");
            return;
        }
    };

    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    // SCREEN_ORIENTATION_SENSOR_LANDSCAPE = 6
    // Allows both normal and reverse landscape based on sensor, but prevents portrait.
    if let Err(e) = env.call_method(
        &activity,
        "setRequestedOrientation",
        "(I)V",
        &[JValue::Int(6)],
    ) {
        log::warn!("setup_android_display: setRequestedOrientation failed: {e:?}");
    }

    let sdk_int = get_sdk_int(&mut env);

    // Set display cutout mode so the app renders into notch/camera cutout areas.
    if sdk_int >= 28 {
        let window = match env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .and_then(|v| v.l())
        {
            Ok(w) => w,
            Err(e) => {
                log::warn!("setup_android_display: failed to get window: {e:?}");
                return;
            }
        };

        let attrs = match env
            .call_method(
                &window,
                "getAttributes",
                "()Landroid/view/WindowManager$LayoutParams;",
                &[],
            )
            .and_then(|v| v.l())
        {
            Ok(a) => a,
            Err(e) => {
                log::warn!("setup_android_display: getAttributes failed: {e:?}");
                return;
            }
        };

        // LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES = 1
        if let Err(e) =
            env.set_field(&attrs, "layoutInDisplayCutoutMode", "I", JValue::Int(1))
        {
            log::warn!("setup_android_display: set cutout mode failed: {e:?}");
        }

        if let Err(e) = env.call_method(
            &window,
            "setAttributes",
            "(Landroid/view/WindowManager$LayoutParams;)V",
            &[JValue::Object(&attrs)],
        ) {
            log::warn!("setup_android_display: setAttributes failed: {e:?}");
        }
    }
}

/// Get the Android SDK version (API level).
fn get_sdk_int(env: &mut jni::JNIEnv<'_>) -> i32 {
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .and_then(|v| v.i())
        .unwrap_or(24)
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
        setup_android_display();
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
