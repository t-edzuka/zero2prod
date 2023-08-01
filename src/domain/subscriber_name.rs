#[derive(Debug)]
pub struct SubscriberName(String);

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SubscriberName {
    pub fn parse(s: String) -> Result<Self, String> {
        if !Self::is_valid_name(s.as_ref()) {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(Self(s))
        }
    }

    fn is_valid_name(s: &str) -> bool {
        use unicode_segmentation::UnicodeSegmentation;

        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let contains_forbidden_characters =
            s.chars().any(|char| forbidden_characters.contains(&char));
        !(is_too_long || is_empty_or_whitespace || contains_forbidden_characters)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;

    use super::*;

    fn valid_names() -> Vec<&'static str> {
        vec!["Ursula Le Guin", "Brian May", "山田太郎"]
    }

    #[test]
    fn valid_names_are_parsed_successfully() {
        for valid_name in valid_names() {
            let name = SubscriberName::parse(valid_name.to_string());
            assert!(name.is_ok());
        }
    }

    #[test]
    fn a_256_grapheme_name_is_valid() {
        let name = "あ".repeat(256);
        let name = SubscriberName::parse(name);
        assert!(name.is_ok());
    }

    #[test]
    fn a_name_longer_than_256_is_rejected() {
        let name = "あ".repeat(257);
        let sut = SubscriberName::parse(name);
        assert_err!(sut);
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn white_space_only_string_is_rejected() {
        let name = " ".repeat(8);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn name_containing_forbidden_characters_is_rejected() {
        let invalid_strings = vec!["/", "(", ")", "\"", "<", ">", "\\", "{", "}"];
        for invalid_string in invalid_strings {
            let name = format!("Brian{}Blade", invalid_string);
            assert_err!(SubscriberName::parse(name));
        }
    }
}
