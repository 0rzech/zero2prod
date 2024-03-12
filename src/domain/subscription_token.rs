use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::iter::repeat_with;

const TOKEN_CHARS: &str = r"[[:alnum:]]";
const TOKEN_LENGTH: usize = 25;

pub fn token_regex() -> String {
    format!(r"{TOKEN_CHARS}{{{TOKEN_LENGTH}}}")
}

fn token_regex_anchored() -> String {
    format!(r"^{}$", token_regex())
}

#[derive(Clone, Debug, Deserialize)]
pub struct SubscriptionToken(Secret<String>);

impl SubscriptionToken {
    pub fn generate() -> Self {
        Self::generate_with_rng(&mut thread_rng())
    }

    fn generate_with_rng(rng: &mut impl Rng) -> Self {
        let token = repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(TOKEN_LENGTH)
            .collect();

        Self(Secret::new(token))
    }

    pub fn parse(s: String) -> Result<Self, String> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(&token_regex_anchored()).unwrap());

        if RE.is_match(&s) {
            Ok(Self(Secret::new(s)))
        } else {
            Err(format!("Invalid subscription token: `{s}`"))
        }
    }
}

impl ExposeSecret<String> for SubscriptionToken {
    fn expose_secret(&self) -> &String {
        self.0.expose_secret()
    }
}

impl TryFrom<String> for SubscriptionToken {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::{token_regex, SubscriptionToken};
    use claims::{assert_err, assert_ok};
    use helpers::{invalid_length_tokens, non_alnum_tokens, valid_tokens};
    use proptest::prelude::proptest;
    use secrecy::ExposeSecret;

    proptest! {
        #[test]
        fn generated_tokens_are_valid(token in valid_tokens()) {
            // when
            let result = SubscriptionToken::parse(token.expose_secret().into());

            // then
            assert_ok!(result);
        }
    }

    proptest! {
        #[test]
        fn valid_tokens_are_parsed_successfully(token in token_regex().as_str()) {
            // when
            let result = SubscriptionToken::parse(token);

            // then
            assert_ok!(result);
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        // given
        let token = "".to_string();

        // when
        let result = SubscriptionToken::parse(token);

        // then
        assert_err!(result);
    }

    proptest! {
        #[test]
        fn tokens_with_non_alphanumeric_characters_are_rejected(token in non_alnum_tokens().as_str()) {
            // when
            let result = SubscriptionToken::parse(token);

            // then
            assert_err!(result);
        }
    }

    proptest! {
        #[test]
        fn tokens_with_invalid_length_are_rejected(token in invalid_length_tokens()) {
            // when
            let result = SubscriptionToken::parse(token);

            // then
            assert_err!(result);
        }
    }

    mod helpers {
        use super::super::TOKEN_LENGTH;
        use crate::domain::SubscriptionToken;
        use proptest::{
            strategy::{NewTree, Strategy, ValueTree},
            test_runner::TestRunner,
        };

        pub fn valid_tokens() -> impl Strategy<Value = SubscriptionToken> {
            ValidTokenStrategy
        }

        #[derive(Debug)]
        struct ValidTokenStrategy;

        impl Strategy for ValidTokenStrategy {
            type Tree = ValidTokenValueTree;
            type Value = SubscriptionToken;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(ValidTokenValueTree(SubscriptionToken::generate_with_rng(
                    runner.rng(),
                )))
            }
        }

        struct ValidTokenValueTree(SubscriptionToken);

        impl ValueTree for ValidTokenValueTree {
            type Value = SubscriptionToken;

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

        pub const FILTERED_LENGTHS: [usize; 2] = [0, TOKEN_LENGTH];

        pub fn non_alnum_tokens() -> String {
            format!(r"[[:^alnum:]]{{{TOKEN_LENGTH}}}")
        }

        pub fn invalid_length_tokens() -> impl Strategy<Value = String> {
            let whence = format!("Invalid token length must not be any of {FILTERED_LENGTHS:?}");
            "[[:alnum:]]*".prop_filter(whence, |v| !FILTERED_LENGTHS.contains(&v.len()))
        }
    }
}
