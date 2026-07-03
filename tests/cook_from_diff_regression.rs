//! The founding regression, absorbed from the former `wicket-guard` crate.
//!
//! An AI coding agent once changed a LICENSE copyright holder to a
//! plausible-but-false human name. The cook_from_diff wedge exists so that diff
//! gets caught at preflight before it can be merged. The rule: identity / legal
//! / ownership claims cannot be inferred from diff context; mutation requires
//! canonical basis, and without it the kernel denies.
//!
//! The cook/diff/surfaces modules live under `examples/cook_from_diff/` (they
//! are a wicket *consumer*, not part of the kernel API). This test compiles
//! those exact source files via `#[path]` so the regression stays runnable
//! under `cargo test` without adding anything to wicket's public lib surface.

#![allow(dead_code)]

#[path = "../examples/cook_from_diff/diff.rs"]
mod diff;
#[path = "../examples/cook_from_diff/surfaces.rs"]
mod surfaces;
#[path = "../examples/cook_from_diff/cook.rs"]
mod cook;

use cook::{cook_diff, CookOpts};
use diff::parse;
use wicket::{StandingClass, SurfaceVerdict};

const FIXTURE: &str = include_str!("fixtures/license_copyright_holder_swap.diff");

#[test]
fn license_copyright_holder_swap_is_refused() {
    let parsed = parse(FIXTURE);
    assert!(
        parsed.files.contains_key("LICENSE"),
        "fixture must touch LICENSE; got files: {:?}",
        parsed.files.keys().collect::<Vec<_>>(),
    );

    let opts = CookOpts {
        actor: "claude-code",
        standing: StandingClass::Execute,
        because: "agent inferred copyright holder from diff context",
    };
    let cooked = cook_diff(&parsed, &opts);
    assert_eq!(cooked.len(), 1, "expected exactly one cooked intent");

    let ci = &cooked[0];
    assert_eq!(ci.path, "LICENSE");
    assert_eq!(ci.sub_target.as_deref(), Some("copyright_holder"));
    assert_eq!(ci.intent.target, "LICENSE:copyright_holder");
    assert_eq!(ci.intent.intended_action, "diff.license_mutation");

    let outcome = wicket::check(&ci.intent);
    assert!(
        matches!(
            outcome.surface_verdict,
            SurfaceVerdict::Denied | SurfaceVerdict::Gap
        ),
        "identity attribution mutation without canonical basis must be \
         denied or gap; got {:?}\n  reason_codes: {:?}",
        outcome.surface_verdict,
        outcome.reason_codes,
    );
}

#[test]
fn unrelated_source_change_is_silent() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
 fn main() {
-    println!(\"hi\");
+    println!(\"hello\");
 }
";
    let parsed = parse(diff);
    let opts = CookOpts {
        actor: "claude-code",
        standing: StandingClass::Execute,
        because: "rename greeting",
    };
    let cooked = cook_diff(&parsed, &opts);
    assert!(
        cooked.is_empty(),
        "non-authority surfaces should not produce intents; got {} intents",
        cooked.len(),
    );
}
