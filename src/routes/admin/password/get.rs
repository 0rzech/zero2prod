use askama_axum::Template;
use axum_messages::Messages;

#[tracing::instrument(name = "Get change password form", skip(messages))]
pub(in crate::routes::admin) async fn change_password_form(
    messages: Messages,
) -> ChangePasswordForm<'static> {
    let flashes = messages.map(|m| m.message).collect();

    ChangePasswordForm {
        page_title: "Change Password",
        current_password_label: "Current Password",
        current_password_placeholder: "Enter current password",
        new_password_label: "New Password",
        new_password_placeholder: "Enter new password",
        new_password_check_label: "Confirm New Password",
        new_password_check_placeholder: "Type the new password again",
        change_password_button: "Change password",
        back_link: "Back",
        flashes,
    }
}

#[derive(Template)]
#[template(path = "web/change_password_form.html")]
pub(in crate::routes::admin) struct ChangePasswordForm<'a> {
    page_title: &'a str,
    current_password_label: &'a str,
    current_password_placeholder: &'a str,
    new_password_label: &'a str,
    new_password_placeholder: &'a str,
    new_password_check_label: &'a str,
    new_password_check_placeholder: &'a str,
    change_password_button: &'a str,
    back_link: &'a str,
    flashes: Vec<String>,
}
