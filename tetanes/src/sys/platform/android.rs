use crate::{
    nes::{Running, renderer::Renderer},
    platform::{BuilderExt, Initialize},
};
use std::path::{Path, PathBuf};
use winit::window::WindowAttributes;

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
        // No command line argument handling for Android for now
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
        self
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
