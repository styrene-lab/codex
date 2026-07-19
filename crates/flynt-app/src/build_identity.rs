//! Compile-time application identity for side-by-side Stable, Candidate, and Dev builds.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildIdentity {
    Stable,
    Candidate,
    Dev,
}

impl BuildIdentity {
    pub fn current() -> Self {
        Self::parse(option_env!("FLYNT_BUILD_IDENTITY").unwrap_or("stable"))
    }

    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "candidate" => Self::Candidate,
            "dev" | "development" => Self::Dev,
            _ => Self::Stable,
        }
    }

    pub const fn app_name(self) -> &'static str {
        match self {
            Self::Stable => "Flynt",
            Self::Candidate => "Flynt Candidate",
            Self::Dev => "Flynt Dev",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Candidate => "Candidate",
            Self::Dev => "Dev",
        }
    }

    pub const fn is_stable(self) -> bool {
        matches!(self, Self::Stable)
    }
}

#[cfg(test)]
mod tests {
    use super::BuildIdentity;

    #[test]
    fn parses_supported_build_identities() {
        assert_eq!(BuildIdentity::parse("stable"), BuildIdentity::Stable);
        assert_eq!(BuildIdentity::parse("Candidate"), BuildIdentity::Candidate);
        assert_eq!(BuildIdentity::parse("development"), BuildIdentity::Dev);
    }

    #[test]
    fn unknown_identity_fails_closed_to_stable() {
        assert_eq!(BuildIdentity::parse("unknown"), BuildIdentity::Stable);
    }

    #[test]
    fn identity_names_are_visibly_distinct() {
        assert_eq!(BuildIdentity::Stable.app_name(), "Flynt");
        assert_eq!(BuildIdentity::Candidate.app_name(), "Flynt Candidate");
        assert_eq!(BuildIdentity::Dev.app_name(), "Flynt Dev");
    }
}
