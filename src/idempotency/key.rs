#[derive(Debug)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        // requirements: 1. Not empty, 2.  Length is less than 50
        let is_some = !s.is_empty();
        let is_less_than_50 = s.len() < 50;
        if is_some && is_less_than_50 {
            Ok(Self(s))
        } else {
            anyhow::bail!(
                "The idempotency key must be \
            not empty and less than 50 characters long."
            )
        }
    }
}

impl From<IdempotencyKey> for String {
    fn from(idempotency_key: IdempotencyKey) -> Self {
        idempotency_key.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
