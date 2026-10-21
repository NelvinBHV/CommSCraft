//! Fixture-only, validated RFC 3261 Via branch identifier.
//! RFC 3261 (June 2002) §8.1.1.7 and §17.1.3.
//! Correlation token only: it does not authenticate a SIP response.

#[derive(Debug, PartialEq, Eq)]
pub struct FixtureRegisterBranch(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureBranchError {
    InvalidBranch,
}

impl FixtureRegisterBranch {
    pub fn new_fixture(value: &str) -> Result<Self, FixtureBranchError> {
        if !value.starts_with("z9hG4bK")
            || value.len() <= 7
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_')
        {
            return Err(FixtureBranchError::InvalidBranch);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbounded_injected_or_non_cookie_branches() {
        for value in ["", "z9hG4bK", "not-a-cookie", "z9hG4bK\r\nX", "z9hG4bK!", "z9hG4bKé"] {
            assert_eq!(
                FixtureRegisterBranch::new_fixture(value),
                Err(FixtureBranchError::InvalidBranch)
            );
        }
        assert!(FixtureRegisterBranch::new_fixture(&format!("z9hG4bK{}", "a".repeat(121))).is_err());
        assert!(FixtureRegisterBranch::new_fixture(&format!("z9hG4bK{}", "a".repeat(121 - 1))).is_ok());
    }
}
