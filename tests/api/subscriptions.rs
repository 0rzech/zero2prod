use crate::helpers::TestApp;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // given
    let app = TestApp::spawn().await;
    let body = "name=Imi%C4%99%20Nazwisko&email=imie.nazwisko%40example.com";

    // when
    let response = app.post_subscriptions(body.into()).await;

    // then
    assert_eq!(response.status(), 200);
    let saved = sqlx::query!("SELECT name, email FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, "Imię Nazwisko");
    assert_eq!(saved.email, "imie.nazwisko@example.com");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("name=Imi%C4%99%20Nazwisko&email=", "empty email"),
        (
            "name=Imi%C4%99%20Nazwisko&email=definitely-not-an-email",
            "invalid email",
        ),
        ("name=&email=imie.nazwisko%40example.com", "empty name"),
        ("name=&email=", "empty both name and email"),
    ];

    for (body, description) in test_cases {
        // when
        let response = app.post_subscriptions(body.into()).await;

        // then
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BAD_REQUEST when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // given
    let app = TestApp::spawn().await;
    let test_cases = vec![
        ("name=Imi%C4%99%20Nazwisko", "missing the email"),
        ("email=imie.nazwisko%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, message) in test_cases {
        // when
        let response = app.post_subscriptions(body.into()).await;

        // then
        assert_eq!(
            response.status(),
            422,
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            message
        );
        let saved = sqlx::query!("SELECT name, email FROM subscriptions")
            .fetch_optional(&app.db_pool)
            .await
            .expect("Failed to fetch unsaved subscription");
        assert!(saved.is_none());
    }
}