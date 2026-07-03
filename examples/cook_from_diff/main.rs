//! Example: cook a unified diff into wicket Intents and check each one.
//!
//! Absorbed from the former `wicket-guard` crate (see that repo's LINEAGE.md).
//! Demonstrates the product wedge: an AI-agent-authored diff touching an
//! authority-bearing surface (LICENSE / NOTICE / CODEOWNERS / SECURITY.md) is
//! cooked into a `bind`-class Intent with the diff as caller-asserted evidence;
//! without canonical basis the kernel denies. This is an EXAMPLE, not a shipped
//! binary — the cook/diff/surfaces capability lives here so it stays exercised
//! (the founding regression is `tests/cook_from_diff_regression.rs`) without
//! growing the wicket kernel's own API surface.
//!
//! Usage: `cargo run --example cook_from_diff -- <path-to.diff>`
//! (reads stdin if no path is given).

// The modules carry the full former-crate API; the example exercises a subset.
#![allow(dead_code)]

mod cook;
mod diff;
mod surfaces;

use std::io::Read;

use cook::{cook_diff, CookOpts};
use diff::parse;
use wicket::StandingClass;

fn main() {
    let arg = std::env::args().nth(1);
    let input = match arg {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {path}: {e}")),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("read stdin");
            buf
        }
    };

    let parsed = parse(&input);
    let opts = CookOpts {
        actor: "example-agent",
        standing: StandingClass::Execute,
        because: "cook_from_diff example",
    };
    let cooked = cook_diff(&parsed, &opts);

    if cooked.is_empty() {
        println!("no authority-bearing surfaces touched — silent (nothing to gate)");
        return;
    }

    for ci in &cooked {
        let outcome = wicket::check(&ci.intent);
        println!(
            "{} [{}] -> {:?}  reasons={:?}",
            ci.path,
            ci.sub_target.as_deref().unwrap_or("-"),
            outcome.surface_verdict,
            outcome.reason_codes,
        );
    }
}
