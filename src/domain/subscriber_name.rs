use once_cell::sync::Lazy;
use serde::Deserialize;
use sqlx::{
    error::BoxDynError,
    postgres::{PgTypeInfo, PgValueRef},
    Decode, Postgres, Type,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Deserialize)]
pub struct SubscriberName(String);

static FORBIDDEN_CHARS: [char; 10] = ['<', '>', '\'', '"', '\\', '(', ')', '{', '}', '/'];
static FORBIDDEN_CHARS_STRING: Lazy<String> = Lazy::new(|| String::from_iter(FORBIDDEN_CHARS));

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        match s {
            _ if s.trim().is_empty() => Err(format!(
                "Subscriber name is empty or contains whitespace only: `{s}`"
            )),
            _ if s.graphemes(true).count() > 256 => {
                Err(format!("`{s}` is longer than 256 graphemes"))
            }
            _ if s.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) => Err(format!(
                "`{s}` contains at least one of forbidden characters: {}",
                *FORBIDDEN_CHARS_STRING
            )),
            _ => Ok(Self(s)),
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Type<Postgres> for SubscriberName {
    fn type_info() -> PgTypeInfo {
        String::type_info()
    }
}

impl<'r> Decode<'r, Postgres> for SubscriberName {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let name = String::decode(value)?;
        Self::parse(name).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::FORBIDDEN_CHARS;
    use crate::domain::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        // given
        let name = "Imię Nazwisko".to_string();

        // when
        let result = SubscriberName::parse(name);

        // then
        assert_ok!(result);
    }

    #[test]
    fn empty_string_is_rejected() {
        // given
        let name = "".to_string();

        // when
        let result = SubscriberName::parse(name);

        // then
        assert_err!(result);
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        // given
        let name = " ".repeat(10);

        // when
        let result = SubscriberName::parse(name);

        // then
        assert_err!(result);
    }

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        // given
        let name = "ę".repeat(256);

        // when
        let result = SubscriberName::parse(name);

        // then
        assert_ok!(result);
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        // given
        let name = "ę".repeat(257);

        // when
        let result = SubscriberName::parse(name);

        // then
        assert_err!(result);
    }

    #[test]
    fn names_containing_invalid_characters_are_rejected() {
        // given
        for name in FORBIDDEN_CHARS {
            let name = name.to_string();

            // when
            let result = SubscriberName::parse(name);

            // then
            assert_err!(result);
        }
    }
}
