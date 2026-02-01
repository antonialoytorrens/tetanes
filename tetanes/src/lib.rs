#![doc = include_str!("../README.md")]
#![doc(
    html_favicon_url = "https://github.com/lukexor/tetanes/blob/main/assets/linux/icon.png?raw=true",
    html_logo_url = "https://github.com/lukexor/tetanes/blob/main/assets/linux/icon.png?raw=true"
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod error;
pub mod logging;
pub mod nes;
pub mod platform;
pub mod sys;
pub mod thread;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use crate::nes::config::Config;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    if let Some(path) = app.internal_data_path() {
        std::env::set_var("TETANES_ANDROID_DATA_DIR", path);
    }

    let config = Config::default();

    if let Err(err) = crate::nes::Nes::run(config, app) {
        log::error!("Failed to run application: {err:?}");
    }
}
