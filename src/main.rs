#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
mod bookmarks;
use crate::bookmarks::{Bookmark, alias_to_bookmark};
use rocket::http::RawStr;
use rocket::response::Redirect;

const DEFAULT_ALIAS: &str = "g";


#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/q/<input>")]
fn redirect(input: &RawStr) -> Redirect {
    let mut splitted = input.splitn(2, "%20");
    let bookmark_alias = splitted.next().unwrap();
    let query = splitted.next().unwrap_or_default();

    let redirect_url = match alias_to_bookmark(bookmark_alias) {
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => alias_to_bookmark(DEFAULT_ALIAS)
            .expect("Default search engine alias was not found!")
            .get_redirect_url(input),
    };

    Redirect::to(redirect_url)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, redirect])
        .launch();
}
