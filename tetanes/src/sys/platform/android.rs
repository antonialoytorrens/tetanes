use crate::{
    nes::{Running, renderer::Renderer},
    platform::{BuilderExt, Initialize},
};
use std::path::{Path, PathBuf};
use winit::window::{Fullscreen, WindowAttributes};

/// Hide the Android navigation bar and status bar using immersive mode.
// Uses setSystemUiVisibility
// You may want to take a look at: https://stackoverflow.com/questions/62577645/android-view-view-systemuivisibility-deprecated-what-is-the-replacement
/// Hide the Android navigation bar and status bar using immersive mode.
///
/// - **API 30+**: Uses `WindowInsetsControllerCompat` from AndroidX Core
///   (≥ 1.6.0-alpha03), the modern replacement for the deprecated
///   `setSystemUiVisibility`.
/// - **API < 30**: Falls back to `setSystemUiVisibility` with immersive-sticky
///   flags.
pub fn hide_navigation_bar_impl() {
    use jni::{
        JavaVM,
        objects::{JObject},
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

    let sdk_int = get_sdk_int(&mut env);

    if sdk_int >= 30 {
        hide_navigation_bar_modern(&mut env, &window, &decor_view);
    } else {
        hide_navigation_bar_legacy(&mut env, &decor_view);
    }
}

// API 30+): WindowInsetsControllerCompat

fn hide_navigation_bar_modern(
    env: &mut jni::JNIEnv<'_>,
    window: &jni::objects::JObject<'_>,
    decor_view: &jni::objects::JObject<'_>,
) {
    use jni::objects::JValue;

    // WindowCompat.setDecorFitsSystemWindows(window, false)
    let window_compat_class = match env.find_class("androidx/core/view/WindowCompat") {
        Ok(c) => c,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to find WindowCompat: {e:?}");
            return;
        }
    };

    if let Err(e) = env.call_static_method(
        &window_compat_class,
        "setDecorFitsSystemWindows",
        "(Landroid/view/Window;Z)V",
        &[JValue::Object(window), JValue::Bool(0)],
    ) {
        log::warn!("hide_navigation_bar: setDecorFitsSystemWindows failed: {e:?}");
        return;
    }

    // Construct WindowInsetsControllerCompat(window, decorView)
    let controller_class =
        match env.find_class("androidx/core/view/WindowInsetsControllerCompat") {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "hide_navigation_bar: failed to find WindowInsetsControllerCompat: {e:?}"
                );
                return;
            }
        };

    let controller = match env.new_object(
        &controller_class,
        "(Landroid/view/Window;Landroid/view/View;)V",
        &[JValue::Object(window), JValue::Object(decor_view)],
    ) {
        Ok(c) => c,
        Err(e) => {
            log::warn!(
                "hide_navigation_bar: failed to create WindowInsetsControllerCompat: {e:?}"
            );
            return;
        }
    };

    // WindowInsetsCompat.Type.systemBars()
    let type_class = match env.find_class("androidx/core/view/WindowInsetsCompat$Type") {
        Ok(c) => c,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to find WindowInsetsCompat.Type: {e:?}");
            return;
        }
    };

    let system_bars = match env
        .call_static_method(&type_class, "systemBars", "()I", &[])
        .and_then(|v| v.i())
    {
        Ok(bits) => bits,
        Err(e) => {
            log::warn!("hide_navigation_bar: failed to call systemBars(): {e:?}");
            return;
        }
    };

    // controller.hide(WindowInsetsCompat.Type.systemBars())
    if let Err(e) = env.call_method(
        &controller,
        "hide",
        "(I)V",
        &[JValue::Int(system_bars)],
    ) {
        log::warn!("hide_navigation_bar: hide() failed: {e:?}");
    }

    // controller.setSystemBarsBehavior(BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)
    const BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE: i32 = 2;

    if let Err(e) = env.call_method(
        &controller,
        "setSystemBarsBehavior",
        "(I)V",
        &[JValue::Int(BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)],
    ) {
        log::warn!("hide_navigation_bar: setSystemBarsBehavior failed: {e:?}");
    }
}

// ── Legacy path (API < 30): setSystemUiVisibility ────────────────────────────

fn hide_navigation_bar_legacy(
    env: &mut jni::JNIEnv<'_>,
    decor_view: &jni::objects::JObject<'_>,
) {
    use jni::objects::JValue;

    const FLAGS: i32 =
        0x0000_0002 | // SYSTEM_UI_FLAG_HIDE_NAVIGATION
        0x0000_0004 | // SYSTEM_UI_FLAG_FULLSCREEN
        0x0000_1000 | // SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        0x0000_0100 | // SYSTEM_UI_FLAG_LAYOUT_STABLE
        0x0000_0200 | // SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
        0x0000_0400;  // SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN

    if let Err(e) = env.call_method(
        decor_view,
        "setSystemUiVisibility",
        "(I)V",
        &[JValue::Int(FLAGS)],
    ) {
        log::warn!("hide_navigation_bar: setSystemUiVisibility failed: {e:?}");
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
        .unwrap_or(26)
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
