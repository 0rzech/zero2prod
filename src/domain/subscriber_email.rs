use serde::Deserialize;
use std::fmt::Display;
use validator::validate_email;

#[derive(Clone, Debug, Deserialize)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("`{s}` email has invalid format"))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claims::{assert_err, assert_ok};
    use helpers::valid_emails;
    use proptest::prelude::proptest;

    proptest! {
        #[test]
        fn valid_emails_are_parsed_successfully(valid_email in valid_emails()) {
            // when
            let result = SubscriberEmail::parse(valid_email);

            // then
            assert_ok!(result);
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        // given
        let email = "".to_string();

        // when
        let result = SubscriberEmail::parse(email);

        // then
        assert_err!(result);
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        // given
        let email = "imie.nazwiskoexample.com".to_string();

        // when
        let result = SubscriberEmail::parse(email);

        // then
        assert_err!(result);
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        // given
        let email = "@xample.com".to_string();

        // when
        let result = SubscriberEmail::parse(email);

        // then
        assert_err!(result);
    }

    mod helpers {
        use fake::{
            faker::internet::en::{FreeEmail, SafeEmail},
            Fake,
        };
        use proptest::{
            prelude::Strategy,
            prop_oneof,
            strategy::{NewTree, ValueTree},
            test_runner::TestRunner,
        };

        pub fn valid_emails() -> impl Strategy<Value = String> {
            // using just `SafeEmailStrategy` would be enough to deliver what's in the book
            prop_oneof![FreeEmailStrategy, SafeEmailStrategy]
        }

        #[derive(Debug)]
        struct FreeEmailStrategy;

        impl Strategy for FreeEmailStrategy {
            type Tree = ValidEmailValueTree;
            type Value = String;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(ValidEmailValueTree(FreeEmail().fake_with_rng(runner.rng())))
            }
        }

        #[derive(Debug)]
        struct SafeEmailStrategy;

        impl Strategy for SafeEmailStrategy {
            type Tree = ValidEmailValueTree;
            type Value = String;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(ValidEmailValueTree(SafeEmail().fake_with_rng(runner.rng())))
            }
        }

        struct ValidEmailValueTree(String);

        impl ValueTree for ValidEmailValueTree {
            type Value = String;

            fn current(&self) -> Self::Value {
                self.0.clone()
            }

            fn simplify(&mut self) -> bool {
                false
            }

            fn complicate(&mut self) -> bool {
                false
            }
        }
    }
}
