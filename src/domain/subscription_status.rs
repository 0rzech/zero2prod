use sqlx::Type;

#[derive(PartialEq, Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
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
