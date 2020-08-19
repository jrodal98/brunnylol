# brunnylol

Like bunnylol, but made with Rust.

## How to use

* Add <https://brunnylol.xyz/search?q=%s> as a search engine, or run your own server. Follow instructions here for setting up a nginx webserver on a VPS: <https://jrodal98.github.io/posts/how-to-deploy-rocket-rust-web-app-on-vps/>
* Search `<alias> <query>`, where `alias` is the shortname for the bookmark (e.g. the alias for Google is g).
* See list of commands on <https://brunnylol.xyz>

## Future changes

* support for more complex bookmarks (e.g. am,electronics \<query\> might specifically search the electronics section for amazon)
* Add more bookmarks
    * facebook
    * instagram
    * UVAcollab
    * sis
    * linkedin
    * dropbox
    * stack overflow
    * pastebin

## Contributing

Submit an issue request with bookmark ideas or add the bookmark in `src/bookmarks.rs` and the alias to bookmark mapping in `src/utils.rs`.
