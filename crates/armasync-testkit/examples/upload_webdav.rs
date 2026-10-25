// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unwrap_used, clippy::print_stdout, clippy::print_stderr)]
//! A manual test server for the upload screen (MIG-042, A11Y protocol). It is a development tool, not a product feature.
//!
//! `cargo run -p armasync-testkit --example upload_webdav` starts a WebDAV server on 127.0.0.1 with a free port. It
//! accepts anonymous plain HTTP only, stores everything in memory and is stopped by typing `q` and Enter (or by Ctrl+C,
//! which ends the process at once). Nothing is written to disk; nothing is sent anywhere else.

use std::io::{BufRead, Write};

use armasync_testkit::webdav::{ManualLimits, start_manual};

/// Prints a line. A closed output pipe (the console window was closed) must not panic the server while it stops.
macro_rules! say {
    ($($arg:tt)*) => {{
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

fn main() {
    let limits = ManualLimits::default();
    let server = start_manual(limits);
    say!("ArmaSync Upload-Prüfserver (nur für die manuelle Prüfung, keine Produktfunktion)");
    say!();
    say!("  Zieladresse für die GUI: http://{}/dav", server.addr);
    say!("  Benutzername und Kennwort leer lassen.");
    say!(
        "  Grenzen: {} MiB je Datei, {} MiB insgesamt, {} Dateien.",
        limits.max_file_bytes / (1024 * 1024),
        limits.max_total_bytes / (1024 * 1024),
        limits.max_files
    );
    say!("  Nur 127.0.0.1, keine Anmeldung, nichts wird auf der Platte gespeichert.");
    say!();
    say!("  Beenden: q und Enter eingeben (oder Strg+C).");

    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.lock().read_line(&mut line) {
            // End of input: nobody can type `q` any more, so the server stops.
            Ok(0) | Err(_) => break,
            Ok(_) if line.trim().eq_ignore_ascii_case("q") => break,
            Ok(_) => {}
        }
    }

    say!();
    say!("Upload-Prüfserver wird beendet.");
    let names = server.file_names();
    say!("  Gespeicherte Dateien: {}", names.len());
    for name in &names {
        let size = server.file(name).map_or(0, |bytes| bytes.len());
        say!("    {name} ({size} Bytes)");
    }
    say!("  Anfragen gesamt: {}", server.seen().len());
    say!("  Schreibvorgänge (PUT): {}", server.puts());
    // Dropping the server stops the accept loop; the process ends with `main`, so no thread is left behind.
    drop(server);
    say!("Beendet. Die Dateien waren nur im Arbeitsspeicher und sind jetzt weg.");
}
