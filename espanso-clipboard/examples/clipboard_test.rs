//! Manual test binary for espanso-clipboard.
//!
//! Works on all platforms; uses `get_clipboard()` so the active backend
//! is selected automatically.
//!
//! Usage: cargo run -p espanso-clipboard --example clipboard_test -- [flags] <command>

use espanso_clipboard::{get_clipboard, Clipboard, ClipboardOperationOptions, ClipboardOptions, ClipboardSnapshot};
use std::thread;
use std::time::Duration;

// --- Argument helpers

struct GlobalFlags {
    /// Force xclip backend (X11 only; ignored on other platforms).
    use_xclip: bool,
    /// Print per-MIME detail in snapshot output.
    show_types: bool,
    /// Positional args after flags are stripped (first element is the subcommand).
    args: Vec<String>,
}

impl GlobalFlags {
    fn clip_opts(&self) -> ClipboardOperationOptions {
        ClipboardOperationOptions {
            use_xclip_backend: self.use_xclip,
        }
    }
}

fn parse_global_flags(args: Vec<String>) -> GlobalFlags {
    let mut use_xclip = false;
    let mut show_types = false;

    let args: Vec<String> = args
        .into_iter()
        .filter(|a| match a.as_str() {
            "--use-xclip" => {
                use_xclip = true;
                false
            }
            "--show-types" => {
                show_types = true;
                false
            }
            _ => true,
        })
        .collect();

    GlobalFlags {
        use_xclip,
        show_types,
        args,
    }
}

// --- Subcommand handlers

fn cmd_get(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    match clipboard.get_text(&flags.clip_opts()) {
        Some(text) => println!("clipboard text: {text:?}"),
        None => println!("clipboard is empty or non-text"),
    }
}

fn cmd_set(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let text = flags
        .args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("hello from espanso-clipboard");
    clipboard
        .set_text(text, &flags.clip_opts())
        .expect("set_text failed");
    println!("set clipboard to: {text:?}");
    thread::sleep(Duration::from_secs(5));
    println!("done (clipboard content may now be lost)");
}

fn cmd_set_html(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let html = "<b>Hello</b> from <em>espanso HTML content</em>";
    let plain = "Hello from espanso plain content";
    clipboard
        .set_html(html, Some(plain), &flags.clip_opts())
        .expect("set_html failed");
    println!("set clipboard HTML: {html:?}");
    println!("         fallback: {plain:?}");
    thread::sleep(Duration::from_secs(5));
    println!("done");
}

fn cmd_set_image(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let path = flags.args.get(1).expect("usage: set-image <path>");
    clipboard
        .set_image(std::path::Path::new(path), &flags.clip_opts())
        .expect("set_image failed");
    println!("set clipboard image from: {path:?}");
    thread::sleep(Duration::from_secs(5));
    println!("done");
}

fn cmd_roundtrip(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let text = flags
        .args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("roundtrip test");
    let clip_opts = flags.clip_opts();
    clipboard.set_text(text, &clip_opts).expect("set_text failed");
    thread::sleep(Duration::from_millis(100));
    match clipboard.get_text(&clip_opts) {
        Some(got) => {
            println!("set:  {text:?}");
            println!("got:  {got:?}");
            if got == text {
                println!("PASS (equal)");
            } else {
                eprintln!("FAIL (mismatch)");
                std::process::exit(1);
            }
        }
        None => {
            eprintln!("FAIL (got nothing)");
            std::process::exit(1);
        }
    }
}

/// Print a summary of the current clipboard snapshot.
fn cmd_snapshot(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let snap = clipboard
        .save_snapshot(&flags.clip_opts())
        .expect("snapshot failed");
    print_snapshot_summary(&snap, flags.show_types);
}

/// Save current clipboard, overwrite with [text], then restore original.
fn cmd_save_restore(clipboard: &dyn Clipboard, flags: &GlobalFlags) {
    let text = flags
        .args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("espanso save-restore test");
    let clip_opts = flags.clip_opts();

    println!("--- snapshotting current clipboard ---");
    let snap = clipboard.save_snapshot(&clip_opts).expect("snapshot failed");
    print_snapshot_summary(&snap, flags.show_types);

    clipboard
        .set_text(text, &clip_opts)
        .expect("set_text failed");
    println!("set clipboard to: {text:?}");
    println!("paste now to verify overwritten content, then press Enter to restore...");
    wait_for_enter();

    let Some(snap) = snap else {
        println!("clipboard snapshot was empty, nothing to restore");
        println!("done");
        return;
    };

    println!("--- restoring snapshot ---");
    clipboard.restore_snapshot(&snap, &clip_opts).expect("restore failed");
    println!("paste now to verify restored content, then press Enter to exit...");
    wait_for_enter();
    println!("done");
}

// --- Display helpers

/// Print a human-readable summary of a [`ClipboardSnapshot`].
fn print_snapshot_summary(snap: &Option<ClipboardSnapshot>, show_types: bool) {
    match snap {
        Some(ClipboardSnapshot::MultiMime(entries)) => {
            println!("snapshot: {} MIME type(s)", entries.len());
            if show_types {
                for (i, entry) in entries.iter().enumerate() {
                    println!("  [{i}] {}  ({} bytes)", entry.mime, entry.payload.len());
                }
            }
        }
        Some(ClipboardSnapshot::TextOnly(text)) => {
            println!("snapshot: text ({} chars)", text.len());
        }
        None => {
            println!("snapshot: (empty)");
            return;
        }
    }
}

fn wait_for_enter() {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
}

fn usage_and_exit(status: i32) -> ! {
    let usage_lines = [
        "USAGE: clipboard_test [flags] <command>",
        "",
        "COMMANDS:",
        "  get",
        "  set [text]",
        "  set-html",
        "  set-image <path>",
        "  roundtrip [text]",
        "  snapshot",
        "  save-restore [text]",
        "",
        "FLAGS:",
        "  --show-types   print MIME types in snapshot output (Mime variant only)",
        "  --use-xclip    force xclip backend (X11 only)",
    ];
    for line in usage_lines {
        eprintln!("{line}");
    }
    std::process::exit(status);
}

// --- Entry point

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let flags = parse_global_flags(raw_args);

    let clipboard: Box<dyn Clipboard> =
        get_clipboard(ClipboardOptions::default()).expect("failed to init clipboard");

    let cmd = flags.args.first().map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "get" => cmd_get(clipboard.as_ref(), &flags),
        "set" => cmd_set(clipboard.as_ref(), &flags),
        "set-html" => cmd_set_html(clipboard.as_ref(), &flags),
        "set-image" => cmd_set_image(clipboard.as_ref(), &flags),
        "roundtrip" => cmd_roundtrip(clipboard.as_ref(), &flags),
        "snapshot" => cmd_snapshot(clipboard.as_ref(), &flags),
        "save-restore" => cmd_save_restore(clipboard.as_ref(), &flags),
        _ => usage_and_exit(1),
    }
}
