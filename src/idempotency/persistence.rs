use super::IdempotencyKey;
use anyhow::anyhow;
use axum::{
    body::{to_bytes, Body},
    http::{Response, StatusCode},
};
use sqlx::{postgres::PgHasArrayType, Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response<Body>),
}

#[tracing::instrument(skip(db_pool, idempotency_key, user_id))]
pub async fn try_processing(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let query = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref()
    );

    if transaction.execute(query).await?.rows_affected() > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let response = get_saved_response(db_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow!("Expected saved response was not found"))?;
        Ok(NextAction::ReturnSavedResponse(response))
    }
}

#[tracing::instrument(skip(db_pool, idempotency_key, user_id))]
pub async fn get_saved_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response<Body>>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code AS "response_status_code!",
            response_headers AS "response_headers!: Vec<HeaderPairRecord>",
            response_body AS "response_body!"
        FROM idempotency
        WHERE
            idempotency_key = $1 AND
            user_id = $2
        "#,
        idempotency_key.as_ref(),
        user_id
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut response = Response::builder().status(status_code);
        for header in r.response_headers {
            response = response.header(header.name, header.value);
        }
        Ok(Some(response.body(Body::from(r.response_body))?))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip(transaction, idempotency_key, user_id, response))]
pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: Response<Body>,
) -> Result<Response<Body>, anyhow::Error> {
    let status_code = response.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response.headers().len());
        for (name, value) in response.headers() {
            let name = name.to_string();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, usize::MAX).await?;

    transaction
        .execute(sqlx::query_unchecked!(
            r#"
            UPDATE idempotency
            SET
                response_status_code = $3,
                response_headers = $4,
                response_body = $5
            WHERE
                user_id = $1 AND
                idempotency_key = $2
            "#,
            user_id,
            idempotency_key.as_ref(),
            status_code,
            headers,
            body.as_ref()
        ))
        .await?;
    transaction.commit().await?;

    let response = Response::from_parts(parts, Body::from(body));
    Ok(response)
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}
