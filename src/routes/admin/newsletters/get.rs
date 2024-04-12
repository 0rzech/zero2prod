use askama_axum::Template;
use axum_messages::Messages;

pub(in crate::routes::admin) async fn newsletter_form(
    messages: Messages,
) -> NewsletterForm<'static> {
    let flashes = messages.map(|m| m.message).collect();

    NewsletterForm {
        title: "Send Newsletter",
        newsletter_title_label: "Newsletter title",
        newsletter_title_placeholder: "Enter newsletter title",
        newsletter_html_label: "Newsletter HTML content",
        newsletter_html_placeholder: "Enter newsletter HTML content",
        newsletter_text_label: "Newsletter text",
        newsletter_text_placeholder: "Enter newsletter text",
        send_newsletter_button: "Send newsletter",
        back_link: "Back",
        flashes,
    }
}

#[derive(Template)]
#[template(path = "web/newsletter_form.html")]
pub(in crate::routes::admin) struct NewsletterForm<'a> {
    title: &'a str,
    newsletter_title_label: &'a str,
    newsletter_title_placeholder: &'a str,
    newsletter_html_label: &'a str,
    newsletter_html_placeholder: &'a str,
    newsletter_text_label: &'a str,
    newsletter_text_placeholder: &'a str,
    send_newsletter_button: &'a str,
    back_link: &'a str,
    flashes: Vec<String>,
}
