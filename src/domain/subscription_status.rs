#[derive(PartialEq)]
pub enum SubscriptionStatus {
    PendingConfirmation,
    Confirmed,
}

impl AsRef<str> for SubscriptionStatus {
    fn as_ref(&self) -> &'static str {
        match self {
            SubscriptionStatus::PendingConfirmation => "pending_confirmation",
            SubscriptionStatus::Confirmed => "confirmed",
        }
    }
}

impl TryFrom<String> for SubscriptionStatus {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_ref() {
            "pending_confirmation" => Ok(SubscriptionStatus::PendingConfirmation),
            "confirmed" => Ok(SubscriptionStatus::Confirmed),
            other => Err(format!(
                "`{other}` is not a valid variant of SubscriptionStatus",
            )),
        }
    }
}
