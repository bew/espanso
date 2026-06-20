/*
 * This file is part of espanso.
 *
 * Copyright (C) 2019-2021 Federico Terzi
 *
 * espanso is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * espanso is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with espanso.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::path::Path;

use anyhow::Result;
use log::{error, info};

#[cfg(target_os = "windows")]
mod win32;

#[cfg(target_os = "linux")]
#[cfg(not(feature = "wayland"))]
mod x11;

#[cfg(target_os = "linux")]
#[cfg(feature = "wayland")]
mod wayland;

#[cfg(target_os = "macos")]
mod cocoa;

/// A single MIME-typed entry in a full clipboard snapshot.
pub struct MimeEntry {
    /// The MIME type string (e.g. `"text/plain;charset=utf-8"`, `"image/png"`).
    pub mime: String,
    /// Raw bytes for this MIME type as provided by the clipboard owner.
    pub payload: Vec<u8>,
}

/// A point-in-time snapshot of the clipboard, used for save/restore around clipboard injection.
pub enum ClipboardSnapshot {
    /// Full multi-MIME snapshot. Empty vec means the clipboard was empty.
    MultiMime(Vec<MimeEntry>),
    /// Text-only snapshot. `None` means the clipboard was empty or non-text.
    TextOnly(String),
}

/// Operations on the system clipboard.
pub trait Clipboard {
    /// Read the current clipboard text, or `None` if empty / non-text.
    fn get_text(&self, options: &ClipboardOperationOptions) -> Option<String>;

    /// Set the clipboard to the given plain text.
    fn set_text(&self, text: &str, options: &ClipboardOperationOptions) -> Result<()>;

    /// Set the clipboard to the image at `image_path` (PNG).
    fn set_image(&self, image_path: &Path, options: &ClipboardOperationOptions) -> Result<()>;

    /// Set the clipboard to HTML content with an optional plain-text fallback.
    fn set_html(
        &self,
        html: &str,
        fallback_text: Option<&str>,
        options: &ClipboardOperationOptions,
    ) -> Result<()>;

    /// Capture the current clipboard state into a [`ClipboardSnapshot`].
    fn save_snapshot(&self, options: &ClipboardOperationOptions) -> Result<Option<ClipboardSnapshot>> {
        // Defaults to basic text-only snapshot support
        if let Some(text) = self.get_text(options) {
            Ok(Some(ClipboardSnapshot::TextOnly(text)))
        } else {
            Ok(None) // no data from clipboard
        }
    }

    /// Restore the clipboard to a previously captured [`ClipboardSnapshot`].
    fn restore_snapshot(
        &self,
        snapshot: &ClipboardSnapshot,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        // Defaults to basic text-only snapshot support
        match snapshot {
            ClipboardSnapshot::TextOnly(text) => {
                self.set_text(text, options)?;
                Ok(())
            }
            ClipboardSnapshot::MultiMime(_) => {
                error!("restore: multi-MIME restore is not supported on this platform");
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct ClipboardOperationOptions {
    pub use_xclip_backend: bool,
}

#[allow(dead_code)]
pub struct ClipboardOptions {
    // Wayland-only
    // The number of milliseconds the wl-clipboard commands are allowed
    // to run before triggering a time-out event.
    wayland_command_timeout_ms: u64,
}

impl Default for ClipboardOptions {
    fn default() -> Self {
        Self {
            wayland_command_timeout_ms: 2000,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_clipboard(_: ClipboardOptions) -> Result<Box<dyn Clipboard>> {
    info!("using Win32Clipboard");
    Ok(Box::new(win32::Win32Clipboard::new()))
}

#[cfg(target_os = "macos")]
pub fn get_clipboard(_: ClipboardOptions) -> Result<Box<dyn Clipboard>> {
    info!("using CocoaClipboard");
    Ok(Box::new(cocoa::CocoaClipboard::new()?))
}

#[cfg(target_os = "linux")]
#[cfg(not(feature = "wayland"))]
pub fn get_clipboard(_: ClipboardOptions) -> Result<Box<dyn Clipboard>> {
    info!("using X11Clipboard");
    Ok(Box::new(x11::X11Clipboard::new()?))
}

#[cfg(target_os = "linux")]
#[cfg(feature = "wayland")]
pub fn get_clipboard(options: ClipboardOptions) -> Result<Box<dyn Clipboard>> {
    info!("using WaylandClipboard");
    Ok(Box::new(wayland::WaylandClipboard::new(options)?))
}
