//! Live smoke test for the Exportmaster native client.
//!
//! Connects, logs in, runs the session-setup handshake against a live
//! DBISAM server. Doesn't yet send a query — verifies connect+login+setup
//! end-to-end, which proves crypto + framing + replay bodies all work.
//!
//! Usage (requires reachable dbsrvr):
//!   cargo run --example em_smoke --features exportmaster -- \
//!     <host> <user> <password> [encrypt_password]
//!
//! Defaults: encrypt_password = "elevatesoft", port = 12005.

#[cfg(feature = "exportmaster")]
fn main() {
    use mrsflow_cli::exportmaster::{Client, ConnOpts};

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: {} <host> <user> <password> [encrypt_password]",
            args[0]
        );
        std::process::exit(2);
    }
    let host = &args[1];
    let user = &args[2];
    let password = &args[3];
    let encrypt = args.get(4).map(String::as_str).unwrap_or("elevatesoft");

    let mut opts = ConnOpts::new(host, user, password);
    opts.encrypt_password = encrypt.to_string();

    eprintln!("Connecting to {}:{}...", opts.host, opts.port);
    let started = std::time::Instant::now();
    match Client::connect_and_login(&opts) {
        Ok(_) => eprintln!(
            "OK: connect + login + session-setup completed in {} ms",
            started.elapsed().as_millis()
        ),
        Err(e) => {
            eprintln!("FAIL: {e:?}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "exportmaster"))]
fn main() {
    eprintln!("Build with --features exportmaster");
    std::process::exit(2);
}
