use askama_axum::Template;
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};

#[tracing::instrument(skip(jar))]
pub(super) async fn login_form(jar: SignedCookieJar) -> (SignedCookieJar, LoginForm<'static>) {
    const FLASH: &str = "_flash";

    let flash = jar.get(FLASH).map(|c| c.value().into());

    (
        jar.remove(Cookie::from(FLASH)),
        LoginForm {
            title: "Login",
            username_label: "Username",
            username_placeholder: "Enter username",
            password_label: "Password",
            password_placeholder: "Enter password",
            submit_label: "Login",
            flash,
            action: "/login",
        },
    )
}

#[derive(Template)]
#[template(path = "web/login_form.html")]
pub(super) struct LoginForm<'a> {
    title: &'a str,
    username_label: &'a str,
    username_placeholder: &'a str,
    password_label: &'a str,
    password_placeholder: &'a str,
    submit_label: &'a str,
    flash: Option<String>,
    action: &'a str,
}
