// SPDX-License-Identifier: AGPL-3.0-only
//! Starts the loopback test repository and prints its address (for manual checks of the GUI and the CLI).
//! `cargo run -p armasync-testkit --example serve [-- trickle]` — stop with Ctrl+C. With `trickle` the large file
//! is sent slowly (about two seconds), so cancellation can be tried by hand.

fn main() {
    let server = armasync_testkit::serve();
    if std::env::args().any(|a| a == "trickle") {
        server.behavior.lock().unwrap().trickle_pbo = true;
    }
    println!("http://{}/repo", server.addr);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
