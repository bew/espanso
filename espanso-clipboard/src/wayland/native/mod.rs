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

use std::io::Read;
use std::path::Path;

use anyhow::Result;
use log::error;
use thiserror::Error;
use wl_clipboard_rs::copy::{
    self as wl_copy, ClipboardType as CopyClipboardType, MimeSource, MimeType as CopyMimeType,
    Options, Seat as CopySeat, ServeRequests, Source,
};
use wl_clipboard_rs::paste::{self, ClipboardType, MimeType as PasteMimeType, Seat};

use crate::{Clipboard, ClipboardOperationOptions, ClipboardSnapshot, MimeEntry};

pub(crate) struct WaylandNativeClipboard;

impl WaylandNativeClipboard {
    pub fn new() -> Result<Self> {
        // Probe that the compositor supports ext/wlr data-control by attempting
        // to list MIME types. An empty clipboard is fine; a missing protocol errors.
        match paste::get_mime_types(ClipboardType::Regular, Seat::Unspecified) {
            Err(paste::Error::MissingProtocol { name, version }) => {
                error!(
                    "Wayland compositor missing required protocol {name} v{version}; \
                     native clipboard unavailable"
                );
                return Err(WaylandNativeClipboardError::ProtocolUnavailable.into());
            }
            // ClipboardEmpty / NoSeats / other errors are fine at init time
            _ => {}
        }

        Ok(Self)
    }
}

impl Clipboard for WaylandNativeClipboard {
    fn get_text(&self, _: &ClipboardOperationOptions) -> Option<String> {
        match paste::get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            PasteMimeType::Text,
        ) {
            Ok((mut reader, _mime)) => {
                let mut text = String::new();
                reader.read_to_string(&mut text).ok()?;
                // wl-paste --no-newline behaviour: strip one trailing newline
                if text.ends_with('\n') {
                    text.pop();
                }
                Some(text)
            }
            Err(paste::Error::ClipboardEmpty) | Err(paste::Error::NoMimeType) => None,
            Err(err) => {
                error!("native get_text failed: {err}");
                None
            }
        }
    }

    fn set_text(&self, text: &str, _: &ClipboardOperationOptions) -> Result<()> {
        let mut opts = Options::new();
        opts.serve_requests(ServeRequests::Unlimited);
        opts.copy(
            Source::Bytes(text.as_bytes().to_vec().into_boxed_slice()),
            CopyMimeType::Text,
        )
        .map_err(|err| {
            error!("native set_text failed: {err}");
            WaylandNativeClipboardError::SetFailed
        })?;
        Ok(())
    }

    fn set_image(&self, image_path: &Path, _: &ClipboardOperationOptions) -> Result<()> {
        if !image_path.exists() || !image_path.is_file() {
            return Err(
                WaylandNativeClipboardError::ImageNotFound(image_path.to_path_buf()).into(),
            );
        }

        let data = std::fs::read(image_path)?;

        let mut opts = Options::new();
        opts.serve_requests(ServeRequests::Unlimited);
        opts.copy(
            Source::Bytes(data.into_boxed_slice()),
            CopyMimeType::Specific("image/png".to_string()),
        )
        .map_err(|err| {
            error!("native set_image failed: {err}");
            WaylandNativeClipboardError::SetFailed
        })?;
        Ok(())
    }

    fn set_html(
        &self,
        html: &str,
        fallback_text: Option<&str>,
        _: &ClipboardOperationOptions,
    ) -> Result<()> {
        let text = fallback_text.unwrap_or(html);

        let mut opts = Options::new();
        opts.serve_requests(ServeRequests::Unlimited);
        opts.copy_multi(vec![
            MimeSource {
                source: Source::Bytes(html.as_bytes().to_vec().into_boxed_slice()),
                mime_type: CopyMimeType::Specific("text/html".to_string()),
            },
            MimeSource {
                source: Source::Bytes(text.as_bytes().to_vec().into_boxed_slice()),
                mime_type: CopyMimeType::Text,
            },
        ])
        .map_err(|err| {
            error!("native set_html failed: {err}");
            WaylandNativeClipboardError::SetFailed
        })?;
        Ok(())
    }

    fn save_snapshot(&self, _: &ClipboardOperationOptions) -> Result<Option<ClipboardSnapshot>> {
        let mimes = match paste::get_mime_types_ordered(ClipboardType::Regular, Seat::Unspecified) {
            Ok(mimes) => mimes,
            Err(paste::Error::ClipboardEmpty) => return Ok(None),
            Err(err) => return Err(err.into()),
        };

        let mut entries = Vec::with_capacity(mimes.len());
        for mime in mimes {
            match paste::get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                PasteMimeType::Specific(&mime),
            ) {
                Ok((mut reader, _)) => {
                    let mut payload = Vec::new();
                    reader.read_to_end(&mut payload)?;
                    // Skip empty payloads — some compositors advertise MIMEs they
                    // cannot actually provide; 0-byte sources confuse restore.
                    if payload.is_empty() {
                        continue;
                    }
                    entries.push(MimeEntry { mime, payload });
                }
                Err(err) => {
                    error!("snapshot: failed to read mime {mime:?}: {err}");
                    return Err(err.into());
                }
            }
        }

        Ok(Some(ClipboardSnapshot::MultiMime(entries)))
    }

    fn restore_snapshot(
        &self,
        snapshot: &ClipboardSnapshot,
        _: &ClipboardOperationOptions,
    ) -> Result<()> {
        match snapshot {
            ClipboardSnapshot::MultiMime(entries) => {
                if entries.is_empty() {
                    log::info!("restore: snapshot is empty, clearing clipboard");
                    wl_copy::clear(CopyClipboardType::Regular, CopySeat::All)?;
                    return Ok(());
                }

                log::info!("restore: {} MIME type(s)", entries.len());
                log::debug!(
                    "restore types: {}",
                    entries
                        .iter()
                        .map(|e| format!("{} ({}B)", e.mime, e.payload.len()))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                let sources: Vec<MimeSource> = entries
                    .iter()
                    .map(|entry| MimeSource {
                        source: Source::Bytes(entry.payload.clone().into_boxed_slice()),
                        mime_type: CopyMimeType::Specific(entry.mime.clone()),
                    })
                    .collect();

                let mut opts = Options::new();
                opts.serve_requests(ServeRequests::Unlimited);
                opts.omit_additional_text_mime_types(true);
                opts.copy_multi(sources).map_err(|err| {
                    error!("restore failed: {err}");
                    WaylandNativeClipboardError::SetFailed
                })?;

                log::info!("restore: clipboard owner transferred to restore serve thread");
                Ok(())
            }
            ClipboardSnapshot::TextOnly(text) => {
                self.set_text(text, &ClipboardOperationOptions::default())
            },
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum WaylandNativeClipboardError {
    #[error("Wayland compositor does not support the data-control protocol")]
    ProtocolUnavailable,

    #[error("clipboard set operation failed")]
    SetFailed,

    #[error("image not found: `{0}`")]
    ImageNotFound(std::path::PathBuf),
}
