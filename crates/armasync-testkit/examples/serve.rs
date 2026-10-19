// SPDX-License-Identifier: AGPL-3.0-only
//! Starts the loopback test repository and prints its address (for manual checks of the GUI and the CLI).
//! `cargo run -p armasync-testkit --example serve` — stop with Ctrl+C.

fn main() {
    let server = armasync_testkit::serve();
    println!("http://{}/repo", server.addr);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
