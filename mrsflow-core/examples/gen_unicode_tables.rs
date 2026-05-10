//! Generate `tools/grammar-fuzz/unicode_tables.pl` — code-point ranges for the
//! Unicode general categories M's identifier grammar references.
//!
//! Both implementations share this single source of truth: the Rust lexer
//! consults `unicode-general-category` directly; the Prolog DCG consults the
//! generated facts. Run when bumping the Unicode version of the crate.
//!
//! Usage: cargo run --example gen_unicode_tables > tools/grammar-fuzz/unicode_tables.pl

use unicode_general_category::{get_general_category, GeneralCategory as G};

fn main() {
    // (predicate name, member general categories) — must agree with
    // is_identifier_start / is_identifier_part in src/lexer/mod.rs.
    let groups: &[(&str, &[G])] = &[
        ("letter_range", &[
            G::UppercaseLetter,
            G::LowercaseLetter,
            G::TitlecaseLetter,
            G::ModifierLetter,
            G::OtherLetter,
            G::LetterNumber,
        ]),
        ("decimal_digit_range", &[G::DecimalNumber]),
        ("connecting_range", &[G::ConnectorPunctuation]),
        ("combining_range", &[G::NonspacingMark, G::SpacingMark]),
        ("formatting_range", &[G::Format]),
    ];

    println!("% Auto-generated. Do not edit.");
    println!("% Source: mrsflow-core/examples/gen_unicode_tables.rs");
    println!("% Regenerate when the unicode-general-category crate version changes.");
    println!();

    for (name, cats) in groups {
        let ranges = collect_ranges(cats);
        println!("% {} ({} ranges)", name, ranges.len());
        for (lo, hi) in &ranges {
            println!("{}(0x{:04X}, 0x{:04X}).", name, lo, hi);
        }
        println!();
    }
}

fn collect_ranges(cats: &[G]) -> Vec<(u32, u32)> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut current: Option<(u32, u32)> = None;

    for cp in 0u32..=0x10FFFF {
        let in_set = match char::from_u32(cp) {
            Some(c) => cats.contains(&get_general_category(c)),
            None => false,
        };
        match (in_set, current) {
            (true, None) => current = Some((cp, cp)),
            (true, Some((lo, _))) => current = Some((lo, cp)),
            (false, Some(r)) => {
                ranges.push(r);
                current = None;
            }
            (false, None) => {}
        }
    }
    if let Some(r) = current {
        ranges.push(r);
    }
    ranges
}
