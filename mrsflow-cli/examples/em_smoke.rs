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

    eprintln!("Smoke test against {}:{}", opts.host, opts.port);
    // count(*) smoke: two documented values per protocol §3. One
    // connection per query — matches the PoC's lifecycle and the v1
    // Exportmaster.Query() shape (each M call is independent).
    for (sql, expected) in [
        ("select count(*) from product", 146_728u32),
        ("select count(*) from analysis", 4_238_476u32),
    ] {
        let started = std::time::Instant::now();
        let mut client = match Client::connect_and_login(&opts) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("FAIL login: {e:?}");
                std::process::exit(1);
            }
        };
        match client.count(sql) {
            Ok(n) => {
                let ok = if n == expected { "OK" } else { "MISMATCH" };
                eprintln!(
                    "{ok}: {sql} = {n} (expected {expected}, in {} ms)",
                    started.elapsed().as_millis()
                );
            }
            Err(e) => {
                eprintln!("FAIL count: {e:?}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(not(feature = "exportmaster"))]
fn main() {
    eprintln!("Build with --features exportmaster");
    std::process::exit(2);
}
