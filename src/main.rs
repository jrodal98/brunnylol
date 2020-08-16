#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
mod bookmarks;
use crate::bookmarks::{alias_to_bookmark, Bookmark};
use rocket::response::Redirect;

const DEFAULT_ALIAS: &str = "g";

#[get("/")]
fn index() -> &'static str {
    "See https://github.com/jrodal98/brunnylol for commands."
}

#[get("/search?<q>")]
fn redirect(q: String) -> Redirect {
    let mut splitted = q.splitn(2, " ");
    let bookmark_alias = splitted.next().unwrap();
    let query = splitted.next().unwrap_or_default();

    let redirect_url = match alias_to_bookmark(bookmark_alias) {
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => alias_to_bookmark(DEFAULT_ALIAS)
            .expect("Default search engine alias was not found!")
            .get_redirect_url(&q),
    };

    Redirect::to(redirect_url)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, redirect])
        .launch();
}
