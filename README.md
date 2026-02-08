[![Rust](https://github.com/jrodal98/brunnylol/actions/workflows/rust.yml/badge.svg)](https://github.com/jrodal98/brunnylol/actions/workflows/rust.yml) [![Docker](https://github.com/jrodal98/brunnylol/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/jrodal98/brunnylol/actions/workflows/docker-publish.yml)

# brunnylol

Like bunnylol, but written in Rust.

A fast, multi-user bookmark management system with personalized shortcuts for quick web navigation.

## Features

- **Personal Bookmarks**: Create custom shortcuts with your own aliases
- **Global Bookmarks**: 20+ built-in shortcuts shared across all users
- **Custom Aliases**: Override or disable any global bookmark
- **Smart Templates**: Variables, optional parameters, and validation
- **Nested Commands**: Bookmarks with sub-commands
- **Import/Export**: YAML and JSON support with URL fetching
- **User Accounts**: Individual logins with isolated bookmark collections
- **Settings**: Configurable default search behavior per user

## TL;DR - Usage

- **Basic:** `alias query` - Type alias and search terms (e.g., `g hello world`)
- **Form mode:** `alias?` - Show interactive form for entering variables
- **Named mode:** `alias$ $key=value` - Provide variables explicitly
- **Nested:** `parent child query` - Use sub-commands (e.g., `dev frontend alice myproject`)

## Quick Start

### Running the Application

```bash
# Clone and build
git clone https://github.com/jrodal98/brunnylol.git
cd brunnylol
cargo build --release

# Run with defaults (port 8000, brunnylol.db)
cargo run --release

# Custom configuration
BRUNNYLOL_PORT=3000 BRUNNYLOL_DB=custom.db cargo run --release

# Or use Docker Compose (see docker-compose.yml for configuration)
docker compose up -d
```

### First Time Setup

1. Navigate to http://localhost:8000
2. Click "Register" (first user becomes admin)
3. Login with your credentials
4. Visit `/settings` to configure default search behavior
5. Start using: `/search?q=<alias> <query>`

### Example Usage

```
# Global bookmarks
/search?q=g hello world           # Google search
/search?q=yt rust programming     # YouTube search

# Personal bookmarks
/search?q=myalias some query

# Suffix syntax (with multi-variable bookmark)
/search?q=myrepo?                 # Form mode
/search?q=myrepo$ $user=alice $repo=myproject  # Named variables
```

## Usage

### Environment Variables

- `BRUNNYLOL_PORT` - Server port (default: 8000)
- `BRUNNYLOL_DB` - Database file path (default: brunnylol.db)

### Bookmark Types

**Standard Bookmarks:**

```yaml
# Simple redirect
alias: help
url: /help
description: Go to brunnylol's help page

# Search template
alias: g
url: https://www.google.com
command: "{url}/search?q={query}"
description: Search google
```

**Nested Bookmarks:**

```yaml
alias: dev
description: Development shortcuts
url: https://example.com
nested:
  - alias: frontend
    description: Frontend repo
    url: https://github.com
    command: "{url}/{user}/{repo}"
  - alias: backend
    description: Backend repo
    url: https://gitlab.com
    command: "{url}/{project}"
```

Usage: `dev frontend alice myproject` → `https://github.com/alice/myproject`

### Suffix Syntax

Use suffix characters to control how bookmarks are invoked:

**Form Mode (`?`):**
Shows an interactive form for entering variables.

```
# Example: bookmark with template "https://example.com/{user}/{repo}"
myrepo alice myproject        # Direct (positional)
myrepo?                       # Form mode - shows form to enter 'user' and 'repo'
```

**Named Mode (`$`):**
Provide variables as `$key=value` pairs.

```
# Direct (positional)
myrepo alice myproject

# Named mode
myrepo$ $user=alice $repo=myproject

# Mixed (named + positional)
myrepo$ $user=alice myproject
```

**Chained Mode (`?$` or `$?`):**
Named variables with form fallback for missing fields.

```
myrepo?$ $user=alice    # Shows form for missing 'repo' field
```

**With Nested Bookmarks:**
Suffix works on parent or child commands.

```
# Example: nested bookmark with sub-commands
dev? backend           # Shows form for dev/backend
dev backend?           # Same result - suffix placement flexible
```

### Template Syntax

- `{var}` - Required variable
- `{var?}` - Optional variable
- `{var=default}` - Default value
- `{var|!encode}` - No URL encoding
- `{var|options[a,b,c]}` - Restrict to specific values

### Managing Bookmarks

**At `/manage`:**

- Create bookmarks manually or import YAML/JSON
- Import from URL or paste content
- Export your bookmarks
- Personal bookmarks override global ones

**At `/settings`:**

- Set default alias for unknown searches
- Change username/password

**Admin Panel (`/admin`):**

- Manage users
- Import/export global bookmarks

## Links

- Live Demo: [https://brunnylol.jrodal.com](https://brunnylol.jrodal.com)
- Help Page: [https://brunnylol.jrodal.com/help](https://brunnylol.jrodal.com/help)

---

## Historical Note

The original implementation of brunnylol was written in 2020 without AI assistance. For those interested in viewing the pre-AI version of the codebase, it has been preserved in the [`legacy`](https://github.com/jrodal98/brunnylol/tree/legacy) branch. The current version incorporates modern development practices and features developed with the assistance of AI-powered coding tools.
