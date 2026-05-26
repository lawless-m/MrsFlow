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
    use mrsflow_cli::exportmaster::{schema, Client, ConnOpts};

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

    // Schema test: parse the column descriptors for `SELECT * FROM product TOP 1`
    // and print the first 12 columns. Per PoC, product has 163 columns
    // starting with CODE/COMMOD/GROUP/DESC1/DESC2/DESC3/DESC4/DESC5/
    // LONGDESC/PRICE/PUNIT/PRICEPER.
    eprintln!("\nSchema test: SELECT * FROM product TOP 1");
    let mut client = match Client::connect_and_login(&opts) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FAIL login: {e:?}");
            std::process::exit(1);
        }
    };
    let raw = match client.query_raw("select * from product top 1") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FAIL query: {e:?}");
            std::process::exit(1);
        }
    };
    eprintln!("  raw response: {} bytes", raw.len());
    match schema::parse(&raw) {
        Ok((cols, end_off)) => {
            eprintln!("  parsed: {} columns, schema region ends at {}", cols.len(), end_off);
            for (i, c) in cols.iter().enumerate().take(12) {
                eprintln!(
                    "    [{i:>3}] {:<14} ord={:<3} type={:?} decl={} max={} row_off={}",
                    c.name, c.ord, c.field_type, c.decl, c.max, c.row_offset
                );
            }
            let expected_n = 163;
            if cols.len() == expected_n {
                eprintln!("  OK: column count {} matches doc expectation", cols.len());
            } else {
                eprintln!(
                    "  MISMATCH: got {} columns, expected {}",
                    cols.len(),
                    expected_n
                );
            }
        }
        Err(e) => {
            eprintln!("  FAIL parse: {e:?}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "exportmaster"))]
fn main() {
    eprintln!("Build with --features exportmaster");
    std::process::exit(2);
}
