//! Build-time generation of the favicon PNGs.
//!
//! `static/favicon.svg` is the artwork source of truth, but browsers
//! and the OpenSearch descriptor also need raster icons
//! (favicon-32.png, favicon-64.png, apple-touch-icon.png). Those
//! PNGs are rendered from the SVG once and stored, base64-encoded,
//! under `assets/` (this project's contribution flow can only push
//! text files, so the binaries travel as text). At build time this
//! script decodes them into `static/` — but only when the target
//! file is missing, so a real PNG placed in `static/` by hand
//! always wins. The generated files are gitignored.
//!
//! To refresh the icons: re-render the PNGs from favicon.svg,
//! re-encode them over the files in `assets/`, and delete the
//! generated PNGs from `static/` so the next build regenerates.

use std::{fs, path::Path};

const ICONS: [(&str, &str); 3] = [
    ("assets/favicon-32.png.b64", "static/favicon-32.png"),
    ("assets/favicon-64.png.b64", "static/favicon-64.png"),
    (
        "assets/apple-touch-icon.png.b64",
        "static/apple-touch-icon.png",
    ),
];

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Minimal standard-alphabet base64 decoder (ignores whitespace).
/// The build has no external dependencies, so this is hand-rolled
/// rather than pulling a crate in as a build-dependency.
fn base64_decode(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &byte in input {
        if byte == b'=' {
            break;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        let value = base64_value(byte).ok_or_else(|| format!("invalid base64 byte: {}", byte))?;
        acc = (acc << 6) | value as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((acc >> nbits) as u8);
        }
    }
    Ok(out)
}

fn main() {
    for (source, target) in ICONS {
        println!("cargo:rerun-if-changed={}", source);
        let target_path = Path::new(target);
        if target_path.exists() {
            continue;
        }
        let encoded = fs::read(source)
            .unwrap_or_else(|e| panic!("favicon generation: cannot read {}: {}", source, e));
        let bytes = base64_decode(&encoded)
            .unwrap_or_else(|e| panic!("favicon generation: cannot decode {}: {}", source, e));
        if !bytes.starts_with(b"\x89PNG") {
            panic!("favicon generation: {} did not decode to a PNG", source);
        }
        fs::write(target_path, &bytes).unwrap_or_else(|e| {
            panic!("favicon generation: cannot write {}: {}", target, e)
        });
        println!(
            "cargo:warning=generated {} from {} ({} bytes)",
            target,
            source,
            bytes.len()
        );
    }
}
