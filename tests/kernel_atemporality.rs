//! Kernel atemporality guard (SPEC §4.5).
//!
//! "Time enters as evidence, not ambient reality."
//!
//! The kernel must not consult wall-clock time; all temporal evaluation
//! runs over caller-supplied fields (`call_timestamp`, `valid_from`,
//! `valid_until`, grant windows, etc.). Wrapper / CLI / cook layers may
//! stamp time; kernel modules may not.
//!
//! This is a cheap denylist test: read each kernel module and grep for
//! known ambient-clock call patterns. It catches the common case (a
//! contributor reaches for `Utc::now()` inside the kernel) without
//! requiring a macro, a feature flag, or a clock-injection abstraction.

const KERNEL_MODULES: &[&str] = &[
    "src/lib.rs",
    "src/model.rs",
    "src/rules.rs",
    "src/grant.rs",
    "src/verdict.rs",
    "src/receipt.rs",
];

const FORBIDDEN_NOW_CALLS: &[&str] = &[
    "Utc::now",
    "Local::now",
    "SystemTime::now",
    "Instant::now",
    "OffsetDateTime::now_utc",
];

#[test]
fn kernel_does_not_consult_ambient_clock() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut violations: Vec<String> = Vec::new();

    for rel in KERNEL_MODULES {
        let path = format!("{manifest_dir}/{rel}");
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("kernel atemporality guard cannot read {path}: {e}"));

        for (i, line) in body.lines().enumerate() {
            let code = strip_line_comment(line);
            for pat in FORBIDDEN_NOW_CALLS {
                if code.contains(pat) {
                    violations.push(format!(
                        "  {rel}:{lineno}  forbidden `{pat}`\n      > {trimmed}",
                        lineno = i + 1,
                        trimmed = line.trim_end(),
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "Kernel atemporality violated (SPEC §4.5).\n\
             The kernel must not consult wall-clock time. Move the call \
             into a wrapper layer (cook.rs, main.rs) and pass the stamped \
             timestamp through the Intent / Evidence fields instead.\n\n\
             Violations:\n{}",
            violations.join("\n")
        );
    }
}

/// Strip a `//` line comment so doc / inline comments that name a
/// forbidden pattern (e.g. "// kernel must not call Utc::now()") do not
/// false-trigger the guard. Does not handle block comments or string
/// literals — kernel modules don't currently contain timestamp patterns
/// in those positions; revisit if that changes.
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}
