use askama_axum::Template;
use axum_messages::Messages;

#[tracing::instrument(name = "Get login form", skip(messages))]
pub(super) async fn login_form(messages: Messages) -> LoginForm<'static> {
    let flashes = messages.map(|m| m.message).collect();

    LoginForm {
        page_title: "Login",
        username_label: "Username",
        username_placeholder: "Enter username",
        password_label: "Password",
        password_placeholder: "Enter password",
        submit_label: "Login",
        flashes,
        action: "/login",
    }
}

#[derive(Template)]
#[template(path = "web/login_form.html")]
pub(super) struct LoginForm<'a> {
    page_title: &'a str,
    username_label: &'a str,
    username_placeholder: &'a str,
    password_label: &'a str,
    password_placeholder: &'a str,
    submit_label: &'a str,
    action: &'a str,
    flashes: Vec<String>,
}
