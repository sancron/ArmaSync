// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unwrap_used)]
//! Starts the loopback test repository and prints its address (for manual checks of the GUI and the CLI).
//! `cargo run -p armasync-testkit --example serve [-- trickle|slow]` — stop with Ctrl+C. With `trickle` the large file
//! is sent slowly (`trickle`: about two seconds, `slow`: about twenty), so cancellation can be tried by hand.

fn main() {
    let server = armasync_testkit::serve();
    if std::env::args().any(|a| a == "trickle") {
        server.behavior.lock().unwrap().trickle_pbo = true;
    }
    if std::env::args().any(|a| a == "slow") {
        server.behavior.lock().unwrap().very_slow_pbo = true;
    }
    println!("http://{}/repo", server.addr);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
