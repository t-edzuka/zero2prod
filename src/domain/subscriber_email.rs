use validator::ValidateEmail;

#[derive(Clone, Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        let is_ok_email = s.validate_email();
        if is_ok_email {
            Ok(Self(s))
        } else {
            Err(s)
        }
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We just forward to the Display implementation of // the wrapped String.
        self.0.fmt(f)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use rand::prelude::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        // https://github.com/LukeMathWalker/zero-to-production/issues/34#issuecomment-1552385593
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        let email = SubscriberEmail::parse(valid_email.0);
        email.is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursulaexample.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@example.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}
