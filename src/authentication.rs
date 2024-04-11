use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Validate credentials", skip(db_pool, credentials))]
pub async fn validate_credentials(
    db_pool: &PgPool,
    credentials: Credentials,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        5sbY1nJpEqWJ9gQP0SvDbw$\
        ZgUSqWDG8XJozXYqOTrah9Ori8FmepJwhTHZMLradFU"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(db_pool, credentials.username).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username"))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(db_pool, username))]
async fn get_stored_credentials(
    db_pool: &PgPool,
    username: String,
) -> Result<Option<(Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(skip(db_pool, user_id, password))]
pub async fn change_password(
    db_pool: &PgPool,
    user_id: Uuid,
    password: Secret<String>,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(db_pool)
    .await
    .context("Failed to change user's password in the database")?;

    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
