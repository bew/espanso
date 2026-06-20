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

pub(crate) mod wlcopy;

use anyhow::Result;
use log::info;

use crate::{Clipboard, ClipboardOperationOptions, ClipboardOptions};
use wlcopy::WlCopyClipboard;

pub struct WaylandClipboard {
    wlcopy: WlCopyClipboard,
}

impl WaylandClipboard {
    pub fn new(options: ClipboardOptions) -> Result<Self> {
        info!("using WlCopyClipboard backend");
        let wlcopy = WlCopyClipboard::new(options)?;

        Ok(Self { wlcopy })
    }
}

impl Clipboard for WaylandClipboard {
    fn get_text(&self, options: &ClipboardOperationOptions) -> Option<String> {
        self.wlcopy.get_text(options)
    }

    fn set_text(&self, text: &str, options: &ClipboardOperationOptions) -> Result<()> {
        self.wlcopy.set_text(text, options)
    }

    fn set_image(
        &self,
        image_path: &std::path::Path,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        self.wlcopy.set_image(image_path, options)
    }

    fn set_html(
        &self,
        html: &str,
        fallback_text: Option<&str>,
        options: &ClipboardOperationOptions,
    ) -> Result<()> {
        self.wlcopy.set_html(html, fallback_text, options)
    }
}
