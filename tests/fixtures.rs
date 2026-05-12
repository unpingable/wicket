//! Fixture-driven doctrine tests.
//!
//! Walks `cases/` (excluding `cases/documentation_only/`) and asserts that
//! every fixture's expected verdict matches `wicket::check`'s output.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use wicket::{BasisStatus, Intent, PrecedenceStatus, StandingStatus, SurfaceVerdict};

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    #[serde(default)]
    documentation_only: bool,
    #[serde(default, rename = "doctrine")]
    _doctrine: Vec<String>,
    intent: Intent,
    #[serde(default)]
    expected: Option<Expected>,
}

#[derive(Debug, Deserialize)]
struct Expected {
    surface_verdict: SurfaceVerdict,
    dimensions: ExpectedDims,
    #[serde(default)]
    must_include_reason_codes: Vec<String>,
    #[serde(default)]
    must_forbid: Vec<String>,
    #[serde(default)]
    must_allow: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDims {
    basis: ExpectedDim<BasisStatus>,
    precedence: ExpectedDim<PrecedenceStatus>,
    standing: ExpectedDim<StandingStatus>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDim<S> {
    status: S,
}

fn cases_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases")
}

fn collect_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
}

#[test]
fn all_executable_fixtures_match_expected() {
    let root = cases_root();
    let mut failures: Vec<String> = Vec::new();
    let mut executed = 0usize;
    let mut documented = 0usize;

    for path in collect_fixtures(&root) {
        let raw = fs::read_to_string(&path).expect("read fixture");
        let fixture: Fixture = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

        if fixture.documentation_only {
            documented += 1;
            continue;
        }

        let Some(expected) = fixture.expected.as_ref() else {
            failures.push(format!(
                "{}: executable fixture has no `expected` block",
                fixture.name
            ));
            continue;
        };

        executed += 1;
        let outcome = wicket::check(&fixture.intent);

        let mut local: Vec<String> = Vec::new();

        if outcome.surface_verdict != expected.surface_verdict {
            local.push(format!(
                "surface_verdict: got {:?}, expected {:?}",
                outcome.surface_verdict, expected.surface_verdict
            ));
        }
        if outcome.dimensions.basis.status != expected.dimensions.basis.status {
            local.push(format!(
                "basis.status: got {:?}, expected {:?}",
                outcome.dimensions.basis.status, expected.dimensions.basis.status
            ));
        }
        if outcome.dimensions.precedence.status != expected.dimensions.precedence.status {
            local.push(format!(
                "precedence.status: got {:?}, expected {:?}",
                outcome.dimensions.precedence.status, expected.dimensions.precedence.status
            ));
        }
        if outcome.dimensions.standing.status != expected.dimensions.standing.status {
            local.push(format!(
                "standing.status: got {:?}, expected {:?}",
                outcome.dimensions.standing.status, expected.dimensions.standing.status
            ));
        }
        for code in &expected.must_include_reason_codes {
            if !outcome.reason_codes.iter().any(|c| c == code) {
                local.push(format!(
                    "missing reason_code {code}; got {:?}",
                    outcome.reason_codes
                ));
            }
        }
        for f in &expected.must_forbid {
            if !outcome.forbidden.iter().any(|c| c == f) {
                local.push(format!(
                    "missing forbidden {f}; got {:?}",
                    outcome.forbidden
                ));
            }
        }
        for a in &expected.must_allow {
            if !outcome.allowed.iter().any(|c| c == a) {
                local.push(format!(
                    "missing allowed {a}; got {:?}",
                    outcome.allowed
                ));
            }
        }

        if !local.is_empty() {
            failures.push(format!(
                "FIXTURE {} ({})\n    {}",
                fixture.name,
                path.display(),
                local.join("\n    ")
            ));
        }
    }

    println!(
        "executed {executed} fixtures, skipped {documented} documentation_only"
    );
    assert!(executed >= 13, "expected at least 13 executable fixtures, got {executed}");

    if !failures.is_empty() {
        panic!(
            "\n{} fixture failures:\n\n{}\n",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

#[test]
fn receipts_are_content_addressed_and_stable() {
    // A given intent must always produce a receipt with the same receipt_id.
    let root = cases_root();
    for path in collect_fixtures(&root) {
        let raw = fs::read_to_string(&path).expect("read fixture");
        let fixture: Fixture = serde_json::from_str(&raw).unwrap();
        if fixture.documentation_only {
            continue;
        }
        let a = wicket::check(&fixture.intent);
        let b = wicket::check(&fixture.intent);
        assert_eq!(
            a.receipt.receipt_id, b.receipt.receipt_id,
            "fixture {}: receipt_id not stable", fixture.name
        );
        assert!(
            a.receipt.receipt_id.starts_with("sha256:"),
            "fixture {}: receipt_id not sha256-prefixed: {}",
            fixture.name, a.receipt.receipt_id
        );
        assert_eq!(
            a.receipt.receipt_id.len(),
            "sha256:".len() + 64,
            "fixture {}: receipt_id wrong length: {}",
            fixture.name, a.receipt.receipt_id
        );
    }
}
