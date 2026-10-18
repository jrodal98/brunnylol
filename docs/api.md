# Brunnylol API — for scripts and AI agents

Brunnylol instances expose a small JSON API for managing **your personal
bookmarks** programmatically: no website login, no HTML, no redirects.
This document is the complete reference. It is also served by every
instance at `GET /api`, so an agent working against a live deployment
can always fetch the version that matches the server.

Base URL: your instance origin, e.g. `https://brunnylol.jrodal.com`.

## Authentication

Every API call (except the OpenSearch endpoints below) needs a
personal API token:

```
Authorization: Bearer brn_<token>
```

- Create and revoke tokens in **Settings → API Tokens** on the website.
- A token is shown **exactly once** at creation; only its SHA-256 hash
  is stored server-side. Treat it like a password.
- A token acts with the full permissions of its owner over that user's
  own bookmarks. It cannot touch other users' bookmarks or the global
  (built-in) bookmarks.
- Missing/invalid/revoked tokens get `401` with a JSON error (below).

## Conventions

- Request and response bodies are JSON (`Content-Type: application/json`).
- Errors are always `{ "error": "<message>" }` with a meaningful status:
  `400` validation, `401` auth, `404` unknown alias, `409` alias already
  exists, `500` server error.
- A bookmark looks like this:

```json
{
  "alias": "hn",
  "type": "templated",
  "url": "https://hn.algolia.com",
  "description": "Search Hacker News",
  "command": "{url}/?q={query}",
  "use_count": 12,
  "last_used_at": "2026-10-17 22:41:03",
  "nested": []
}
```

- `type` is `simple`, `templated`, or `nested`. You never send it:
  on create/update it is **inferred** — `nested` present → `nested`,
  non-empty `command` → `templated`, otherwise `simple`.
- `command` is the URL template. It may use `{url}` (the bookmark's
  base URL), `{query}` (the user's search text), named variables like
  `{user}`, and pipeline/map/options syntax as documented on the
  instance's Help page. `null` means a plain redirect bookmark.
- `nested` lists sub-commands of a nested bookmark:
  `{ "alias", "url", "description", "command" }`.
- `use_count` / `last_used_at` are read-only usage stats (searches
  through the bookmark, counted by the server).

## Endpoints

### `GET /api/whoami`

Verify a token and see whose it is.

```bash
curl https://brunnylol.jrodal.com/api/whoami \
  -H "Authorization: Bearer brn_YOUR_TOKEN"
```

```json
{ "username": "jacob", "is_admin": true }
```

### `GET /api/bookmarks`

List all of your personal bookmarks.

```bash
curl https://brunnylol.jrodal.com/api/bookmarks \
  -H "Authorization: Bearer brn_YOUR_TOKEN"
```

Response: `{ "bookmarks": [ <bookmark>, ... ] }`.

### `GET /api/bookmarks/{alias}`

Fetch one bookmark (same object shape as in the list).

### `POST /api/bookmarks`

Create a bookmark. Returns `201` with the stored bookmark.

```bash
curl -X POST https://brunnylol.jrodal.com/api/bookmarks \
  -H "Authorization: Bearer brn_YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "hn",
    "url": "https://hn.algolia.com",
    "description": "Search Hacker News",
    "command": "{url}/?q={query}"
  }'
```

Fields: `alias` (required, 1–50 chars, unique among your bookmarks),
`url` (required unless `nested` is given; must be http/https),
`description` (required), `command` (optional), `nested` (optional,
max 100 children, each validated like a bookmark). Creating a
personal bookmark with the same alias as a built-in is allowed — it
overrides the built-in for you.

### `PUT /api/bookmarks/{alias}`

Replace a bookmark's `url`, `description`, and `command` (send
`"command": null` or omit it for a plain redirect). If `nested` is
present it **replaces all sub-commands**; if absent, sub-commands are
left alone. The alias itself cannot be renamed — delete and recreate.
Returns the updated bookmark.

### `DELETE /api/bookmarks/{alias}`

Delete a bookmark (and its sub-commands and usage stats). Returns
`204 No Content`; `404` if the alias doesn't exist.

## OpenSearch (no token needed)

These endpoints serve browser search-engine integration and are
unauthenticated; alias suggestions personalize via the normal session
cookie when the caller is signed in, and built-ins only otherwise.

- `GET /opensearch.xml` — OpenSearch descriptor for the instance
  (search + suggestions templates).
- `GET /api/suggest?q=<text>` — alias completions in OpenSearch
  suggestions format:
  `[query, [aliases], [descriptions], [search URLs]]`. Matching uses
  the first word of `q` (the alias being typed).

## Tips for agents

- Check your token first with `GET /api/whoami`.
- List before you write: `GET /api/bookmarks` shows what exists,
  including current `use_count`s — prefer updating an existing alias
  over creating a near-duplicate.
- A `409` means the alias is taken (by the user, not necessarily by a
  built-in). A `404` on PUT/DELETE means there is no *personal*
  bookmark with that alias — built-ins can't be edited via the API,
  but creating a personal bookmark with the same alias overrides one.
- To make a bookmark searchable, set `command` to a template
  containing `{query}`; to make a plain shortcut, omit `command`.
