mod util;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // given
    let client = reqwest::Client::new();
    let body = "name=Imi%C4%99%20Nazwisko&email=imi%C4%99.nazwisko%40example.com";
    let app = util::spawn_app().await;
    let url = util::url(app.address, "subscriptions");

    // when
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    // then
    assert_eq!(response.status(), 200);
    let saved = sqlx::query!("SELECT name, email FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.name, "Imię Nazwisko");
    assert_eq!(saved.email, "imię.nazwisko@example.com");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // given
    let client = reqwest::Client::new();
    let app = util::spawn_app().await;
    let url = util::url(app.address, "subscriptions");
    let test_cases = vec![
        ("name=Imi%C4%99%20Nazwisko", "missing the email"),
        ("email=imi%C4%99.nazwisko%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, message) in test_cases {
        // when
        let response = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

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
            .expect("Failed to fetch unsaved subscription.");
        assert!(saved.is_none());
    }
}
