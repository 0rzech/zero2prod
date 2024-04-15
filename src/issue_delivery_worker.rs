use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_pg_connection_pool,
};
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use std::time::Duration;
use tracing::Span;
use uuid::Uuid;

pub async fn run_worker_until_stopped(config: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_pg_connection_pool(&config.database);
    let email_client = config.email_client.client();
    worker_loop(&connection_pool, &email_client).await
}

async fn worker_loop(db_pool: &PgPool, email_client: &EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(db_pool, email_client).await {
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty),
    err
)]
pub async fn try_execute_task(
    db_pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    if let Some((transaction, issue_id, email)) = dequeue_task(db_pool).await? {
        Span::current()
            .record("newsletter_issue_id", issue_id.to_string())
            .record("subscriber_email", email.clone());

        match SubscriberEmail::parse(email.clone()) {
            Ok(email) => {
                let issue = get_issue(db_pool, issue_id).await?;
                if let Err(e) = email_client
                    .send_email(
                        &email,
                        &issue.title,
                        &issue.html_content,
                        &issue.text_content,
                    )
                    .await
                {
                    tracing::error!(
                        error_cause_chain = ?e,
                        error.message = %e,
                        "Failed to deliver issue to a confirmed subscriber. Skipping."
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error_cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. \
                    Their email is invalid."
                );
            }
        }

        delete_task(transaction, issue_id, &email).await?;

        Ok(ExecutionOutcome::TaskCompleted)
    } else {
        Ok(ExecutionOutcome::EmptyQueue)
    }
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let query = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    );

    match transaction.fetch_optional(query).await? {
        Some(row) => Ok(Some((
            transaction,
            row.try_get("newsletter_issue_id")?,
            row.try_get("subscriber_email")?,
        ))),
        None => Ok(None),
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        issue_id,
        email
    );

    transaction.execute(query).await?;
    transaction.commit().await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn get_issue(db_pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        issue_id,
    )
    .fetch_one(db_pool)
    .await?;

    Ok(issue)
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}
