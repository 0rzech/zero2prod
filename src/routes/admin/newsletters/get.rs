use askama_axum::Template;
use axum_messages::Messages;

pub(in crate::routes::admin) async fn newsletter_form(
    messages: Messages,
) -> NewsletterForm<'static> {
    let flashes = messages.map(|m| m.message).collect();

    NewsletterForm {
        page_title: "Send Newsletter",
        title_label: "Newsletter title",
        title_placeholder: "Enter newsletter title",
        html_content_label: "Newsletter HTML content",
        html_content_placeholder: "Enter newsletter HTML content",
        text_content_label: "Newsletter text",
        text_content_placeholder: "Enter newsletter text",
        send_newsletter_button: "Send newsletter",
        back_link: "Back",
        flashes,
    }
}

#[derive(Template)]
#[template(path = "web/newsletter_form.html")]
pub(in crate::routes::admin) struct NewsletterForm<'a> {
    page_title: &'a str,
    title_label: &'a str,
    title_placeholder: &'a str,
    html_content_label: &'a str,
    html_content_placeholder: &'a str,
    text_content_label: &'a str,
    text_content_placeholder: &'a str,
    send_newsletter_button: &'a str,
    back_link: &'a str,
    flashes: Vec<String>,
}
