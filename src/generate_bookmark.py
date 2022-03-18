#!/usr/bin/env python3
# www.jrodal.dev

# This script generates basic structs for bookmarks and adds a new alias for
# the bookmark. It doesn't check if the bookmark is a valid struct name, so don't
# name your bookmark something stupid...

import os
import sys
import subprocess

# define some important file variables
BM_FILENAME = "bookmarks.rs"
BM_START_STR = "\n// START OF STRUCT IMPLEMENTATIONS (DO NOT DELETE THIS LINE)"
ALIAS_FILENAME = "utils.rs"
ALIAS_START_STR = "// END OF ALIAS IMPLEMENTATIONS (DO NOT DELETE THIS LINE)"

# Locate the relevant files
cwd = os.getcwd()
if os.path.isfile(os.path.join(cwd, BM_FILENAME)):
    bm_path = os.path.join(cwd, BM_FILENAME)
    alias_path = os.path.join(cwd, ALIAS_FILENAME)
else:
    bm_path = os.path.join(cwd, "src", BM_FILENAME)
    alias_path = os.path.join(cwd, "src", ALIAS_FILENAME)

# quit if files could not be located
if not os.path.isfile(bm_path) or not os.path.isfile(alias_path):
    print(
        "Could not find {bm_filename} or {alias_filename}. Did you run python src/generate_bookmark.py?",
        file=sys.stderr,
    )

with open(bm_path, "r") as f:
    bm_file_text = f.read()

struct_name = ""
while not struct_name:
    struct_name = input("Name of struct: (e.g.: GoogleCalendar): ")
    if f"pub struct {struct_name};" in bm_file_text:
        print(f"{struct_name} already exists. Pick a different name.")
        struct_name = ""

description = input("Enter bookmark description: ")

base_url = input("Enter base url of bookmark (e.g: https://www.google.com): ")
query_url = input(
    "Enter query url with %s or hit enter to continue (e.g. https://www.google.com/search?q=%s): "
)

query_overrides = []
while to_replace := input("Enter query token to override or hit enter to continue: "):
    replace_with = input("Enter override value: ")
    query_overrides.append(f'"{to_replace}" => "{replace_with}",')


query_override_str = "\n".join(query_overrides)
query_override_code = (
    f"""
    fn override_query<'a>(&self, query: &'a str) -> &'a str {{
        match query {{
            {query_override_str}
            _ => query,
        }}
    }}
"""
    if query_overrides
    else ""
)


urls = f'"{base_url}",'
if query_url:
    urls += f'"{query_url}",'

struct_code = f"""pub struct {struct_name};
{BM_START_STR}


impl Bookmark for {struct_name} {{
    fn urls(&self) -> Vec<&'static str> {{
        vec![{urls}]
    }}

    fn description(&self) -> &'static str {{
        "{description}"
    }}

    {query_override_code}
}}"""

with open(alias_path, "r") as f:
    alias_file_text = f.read()

alias_name = ""
while not alias_name:
    alias_name = input("Name of alias: (e.g.: g): ")
    if f'"{alias_name}" =>' in alias_file_text:
        print(f"{alias_name} already exists. Pick a different name.")
        alias_name = ""

alias_code = (
    f'"{alias_name}" => Box::new(bookmarks::{struct_name}),\n        {ALIAS_START_STR}'
)


with open(bm_path, "w") as f:
    f.write(bm_file_text.replace(BM_START_STR, struct_code))
with open(alias_path, "w") as f:
    f.write(alias_file_text.replace(ALIAS_START_STR, alias_code))

try:
    subprocess.run(["cargo", "fmt"]).check_returncode()
except subprocess.CalledProcessError:
    print("Error when formatting code with cargo. Check if the code compiles.")
except Exception:
    print("Error running `cargo fmt`. If you don't have cargo, ignore this message.")
