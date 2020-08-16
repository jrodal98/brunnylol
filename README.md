# brunnylol

Like bunnylol, but made with Rust.

## How to use

* Add <https://brunnylol.xyz/search?q=%s> as a search engine, or run your own server. Follow instructions here for setting up a nginx webserver on a VPS: <https://jrodal98.github.io/posts/how-to-deploy-rocket-rust-web-app-on-vps/>

## Commands

More details + commands to come soon. (x|y) means that using x or using y works.

* g \<query\>: Search google
* (d|ddg) \<query\>: Search duckduckgo
* b \<query\>: Search bing
* yt \<query\>: Search youtube
* wiki \<query\>: Search wikipedia
* aw \<query\>: Search the arch wiki
* gh \<query\>: Search github
* def \<query\>: Define a word with dictionary.com
* genius \<query\>: search for lyrics on genius.com
* reddit \<query\>: go to a subreddit
* time: Look at time zone information


## Future changes

* support for more complex bookmarks (e.g. am,electronics \<query\> might specifically search the electronics section for amazon)
* Add more bookmarks
    * facebook
    * instagram
    * UVAcollab
    * sis
    * linkedin
    * dropbox
    * my website
    * stack overflow
    * pastebin

## Contributing

Submit an issue request with bookmark ideas or add them yourself in the `src/bookmarks.rs` file.
