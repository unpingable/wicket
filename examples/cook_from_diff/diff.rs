//! Minimal unified-diff parser.
//!
//! Just enough to extract per-file added/removed lines for surface
//! classification. Not a general-purpose parser: does not validate
//! hunk arithmetic, does not handle binary patches, does not track
//! line numbers. If we need any of that, swap in the `unidiff` crate.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct ParsedDiff {
    pub files: BTreeMap<String, FileChanges>,
}

#[derive(Debug, Clone, Default)]
pub struct FileChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

pub fn parse(input: &str) -> ParsedDiff {
    let mut out = ParsedDiff::default();
    let mut current: Option<String> = None;

    for line in input.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            let path = rest.split(" b/").next().unwrap_or(rest).to_string();
            current = Some(path);
            continue;
        }
        if line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@")
            || line.starts_with("index ")
            || line.starts_with("new file mode")
            || line.starts_with("deleted file mode")
            || line.starts_with("similarity index")
            || line.starts_with("rename from")
            || line.starts_with("rename to")
        {
            continue;
        }
        let Some(path) = current.as_ref() else {
            continue;
        };
        let entry = out.files.entry(path.clone()).or_default();
        if let Some(rest) = line.strip_prefix('+') {
            entry.added.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix('-') {
            entry.removed.push(rest.to_string());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_diff() {
        let diff = "\
diff --git a/LICENSE b/LICENSE
index abc..def 100644
--- a/LICENSE
+++ b/LICENSE
@@ -1,3 +1,3 @@
 context line
-old line
+new line
";
        let parsed = parse(diff);
        let changes = parsed.files.get("LICENSE").expect("LICENSE present");
        assert_eq!(changes.removed, vec!["old line"]);
        assert_eq!(changes.added, vec!["new line"]);
    }
}
