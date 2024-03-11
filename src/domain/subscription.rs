use super::{SubscriberEmail, SubscriberName, SubscriptionStatus};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(FromRow)]
pub struct Subscription {
    pub id: Uuid,
    #[sqlx(try_from = "String")]
    pub email: SubscriberEmail,
    #[sqlx(try_from = "String")]
    pub name: SubscriberName,
    pub subscribed_at: OffsetDateTime,
    #[sqlx(try_from = "String")]
    pub status: SubscriptionStatus,
}
