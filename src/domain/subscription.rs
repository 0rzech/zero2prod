use super::{SubscriberEmail, SubscriberName, SubscriptionStatus};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub subscribed_at: OffsetDateTime,
    pub status: SubscriptionStatus,
}
