use crate::{
    session::state::TypedSession,
    utils::{e500, HttpError},
};
use axum::response::Redirect;
use axum_messages::Messages;

#[tracing::instrument(skip(session, messages))]
pub(super) async fn log_out(
    session: TypedSession,
    messages: Messages,
) -> Result<Redirect, HttpError<anyhow::Error>> {
    if session.get_user_id().await.map_err(e500)?.is_some() {
        session.flush().await.map_err(e500)?;
        messages.info("You have successfully logged out.");
    }

    Ok(Redirect::to("/login"))
}
