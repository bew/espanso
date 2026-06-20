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

pub(crate) mod native;
pub(crate) mod wlcopy;

use anyhow::Result;
use log::{info, warn};

use crate::{Clipboard, ClipboardOperationOptions, ClipboardOptions, ClipboardSnapshot};
use native::WaylandNativeClipboard;
use wlcopy::WlCopyClipboard;

pub struct WaylandClipboard {
    native: Option<WaylandNativeClipboard>,
    wlcopy: WlCopyClipboard,
}

impl WaylandClipboard {
    pub fn new(options: ClipboardOptions) -> Result<Self> {
        let native = match WaylandNativeClipboard::new() {
            Ok(backend) => {
                info!("using WaylandNativeClipboard backend");
                Some(backend)
            }
            Err(err) => {
                warn!("native Wayland clipboard unavailable: {err}");
                info!("using WlCopyClipboard backend");
                None
            }
        };
        let wlcopy = WlCopyClipboard::new(options)?;
        Ok(Self { native, wlcopy })
    }

    fn use_native(&self, options: &ClipboardOperationOptions) -> bool {
        self.native.is_some() && !options.use_wlcopy_backend
    }
}

impl Clipboard for WaylandClipboard {
    fn get_text(&self, options: &ClipboardOperationOptions) -> Option<String> {
        if self.use_native(options) {
            self.native.as_ref().unwrap().get_text(options)
        } else {
            self.wlcopy.get_text(options)
        }
    }

    fn set_text(&self, text: &str, options: &ClipboardOperationOptions) -> Result<()> {
        if self.use_native(options) {
            self.native.as_ref().unwrap().set_text(text, options)
        } else {
            self.wlcopy.set_text(text, options)
        }
    }

    fn set_image(
        &self,
        image_path: &std::path::Path,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        if self.use_native(options) {
            self.native.as_ref().unwrap().set_image(image_path, options)
        } else {
            self.wlcopy.set_image(image_path, options)
        }
    }

    fn set_html(
        &self,
        html: &str,
        fallback_text: Option<&str>,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        if self.use_native(options) {
            self.native
                .as_ref()
                .unwrap()
                .set_html(html, fallback_text, options)
        } else {
            self.wlcopy.set_html(html, fallback_text, options)
        }
    }

    fn save_snapshot(&self, options: &ClipboardOperationOptions) -> Result<Option<ClipboardSnapshot>> {
        if self.use_native(options) {
            self.native.as_ref().unwrap().save_snapshot(options)
        } else {
            self.wlcopy.save_snapshot(options)
        }
    }

    fn restore_snapshot(
        &self,
        snapshot: &ClipboardSnapshot,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        if self.use_native(options) {
            self.native.as_ref().unwrap().restore_snapshot(snapshot, options)
        } else {
            self.wlcopy.restore_snapshot(snapshot, options)
        }
    }
}
