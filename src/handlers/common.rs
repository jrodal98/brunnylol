// Common HTMX partial templates shared across handlers

use askama::Template;

#[derive(Template)]
#[template(path = "partials/error.html")]
pub struct ErrorTemplate<'a> {
    pub message: &'a str,
}

#[derive(Template)]
#[template(path = "partials/success.html")]
pub struct SuccessTemplate<'a> {
    pub message: &'a str,
}

#[derive(Template)]
#[template(path = "partials/success_with_link.html")]
pub struct SuccessWithLinkTemplate<'a> {
    pub message: &'a str,
    pub link: &'a str,
    pub link_text: &'a str,
}
