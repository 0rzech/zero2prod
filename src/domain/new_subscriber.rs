use super::{SubscriberEmail, SubscriberName};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
