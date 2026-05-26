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

            // Row-parser smoke: find the first plausible row start in the
            // post-schema region and decode it. PoC pattern-matches row
            // starts via null-flag bytes; we mimic that for v1, decoding
            // the first hit and printing the first 12 columns' values.
            let row_size = cols.last().unwrap().row_offset as usize
                + cols.last().unwrap().max as usize
                + 1;
            eprintln!("  expected row width: {} bytes", row_size);
            // Search post-schema region for a plausible row start.
            // Heuristic from PoC: every column's null-flag byte must be
            // 0x00 or 0x01, AND first column (CODE) starts with a digit
            // (CUSTOMER) or '*' (product). For product CODE the alphabet
            // is broader — relax to "first byte non-null AND CODE
            // null-flag = 0x01".
            let search_start = end_off;
            let mut decoded_first = None;
            for candidate in search_start..raw.len().saturating_sub(row_size + 25) {
                let cells_ok = cols.iter().all(|c| {
                    let p = candidate + 25 + c.row_offset as usize;
                    p < raw.len() && (raw[p] == 0 || raw[p] == 1)
                });
                if !cells_ok {
                    continue;
                }
                // Try to decode; first successful decode wins.
                let record_end = candidate + 25 + row_size;
                if record_end > raw.len() {
                    continue;
                }
                if let Ok(cells) = mrsflow_cli::exportmaster::row::decode_record(
                    &raw[candidate..record_end],
                    &cols,
                ) {
                    // Sanity: first cell should be a non-empty Text value
                    // (CODE is mandatory NOT NULL on product).
                    if let mrsflow_cli::exportmaster::row::CellValue::Text(s) = &cells[0] {
                        if !s.is_empty() {
                            decoded_first = Some((candidate, cells));
                            break;
                        }
                    }
                }
            }
            match decoded_first {
                Some((off, cells)) => {
                    eprintln!("  decoded first row at offset {}:", off);
                    for (i, (col, cell)) in cols.iter().zip(cells.iter()).enumerate().take(12) {
                        eprintln!("    [{i:>2}] {:<14} = {:?}", col.name, cell);
                    }
                }
                None => {
                    eprintln!("  WARN: no decodable row found in post-schema region");
                    // Dump the first 256 bytes after the schema region
                    // so we can see whether row data is even here.
                    eprintln!("  post-schema dump ({} bytes total starting at offset {}):", raw.len() - end_off, end_off);
                    let tail = &raw[end_off..(end_off + 256).min(raw.len())];
                    for (i, chunk) in tail.chunks(32).enumerate() {
                        let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
                        let ascii: String = chunk
                            .iter()
                            .map(|&b| if (32..127).contains(&b) { b as char } else { '.' })
                            .collect();
                        eprintln!("    +{:04x}: {hex} {ascii}", end_off + i * 32);
                    }
                }
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
