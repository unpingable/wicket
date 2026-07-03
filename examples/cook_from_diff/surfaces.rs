//! Classify file paths into authority-bearing surface categories.
//!
//! These are surfaces where unauthorized mutation by an AI agent is the
//! failure mode the wedge is built to prevent. Identity / legal /
//! ownership claims cannot be inferred from diff context.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoritySurface {
    License,
    Notice,
    Codeowners,
    SecurityPolicy,
}

impl AuthoritySurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::License => "license",
            Self::Notice => "notice",
            Self::Codeowners => "codeowners",
            Self::SecurityPolicy => "security_policy",
        }
    }
}

pub fn classify_path(path: &str) -> Option<AuthoritySurface> {
    let basename = path.rsplit('/').next().unwrap_or(path);
    let upper = basename.to_ascii_uppercase();
    match upper.as_str() {
        "LICENSE" | "LICENSE.MD" | "LICENSE.TXT" | "COPYING" | "COPYING.MD" => {
            Some(AuthoritySurface::License)
        }
        "NOTICE" | "NOTICE.MD" | "NOTICE.TXT" => Some(AuthoritySurface::Notice),
        "CODEOWNERS" => Some(AuthoritySurface::Codeowners),
        "SECURITY.MD" => Some(AuthoritySurface::SecurityPolicy),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_canonical_names() {
        assert_eq!(classify_path("LICENSE"), Some(AuthoritySurface::License));
        assert_eq!(classify_path("NOTICE"), Some(AuthoritySurface::Notice));
        assert_eq!(
            classify_path(".github/CODEOWNERS"),
            Some(AuthoritySurface::Codeowners),
        );
        assert_eq!(
            classify_path("SECURITY.md"),
            Some(AuthoritySurface::SecurityPolicy),
        );
        assert_eq!(classify_path("src/main.rs"), None);
    }
}
