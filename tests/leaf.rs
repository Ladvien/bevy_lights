//! **The crate boundary, enforced.**
//!
//! `bevy_lights` is a lighting library any Bevy game can take. It depends on Bevy and on the
//! optional serde derives, and on nothing else: no engine crate beyond Bevy, no physics, no sibling
//! ladvien crate at all. It must also never learn about a particular game.
//!
//! The source scan forbids more than crate names. `CameraFocus`, `Thomas`, `LUMEN_SCALE` and
//! `ActivePalette` are the specific temptations from the game this crate was extracted from: each
//! one would compile, and would silently couple the library to that game's camera marker, its
//! player, its unit scale or its palette resource. The crate's generics and its `FlameLook`
//! resource exist so that none of them is needed.

// Tests are allowed to panic: a failed assertion and a missing fixture are the same thing here, and
// `AGENTS.md`'s no-`unwrap`/`expect`/`panic!` rule is about library code.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

/// Everything this crate is allowed to depend on.
const ALLOWED_DEPS: &[&str] = &["bevy", "serde"];

/// Names that would mean the boundary has been crossed.
const FORBIDDEN_DEP_MARKERS: &[&str] = &[
    "avian",
    "foundation_vs_slop",
    "sanctuary",
    "Sanctuary",
    "MainCamera",
    "SquadMember",
    "MenuState",
    "CameraFocus",
    "Thomas",
    "LUMEN_SCALE",
    "ActivePalette",
];

fn crate_root() -> PathBuf {
    // Cargo runs a test binary with the cwd set to its own package root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `path` relative to the crate root, with `/` separators.
fn relative(path: &Path) -> String {
    path.strip_prefix(crate_root()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// Strip `//` line comments and `/* */` blocks, so a doc comment *about* a game is not mistaken for
/// a mention of one in code.
///
/// Deliberately simple: it does not track string literals, so a `"//"` inside a string ends the line
/// early. That is acceptable because the output only feeds substring searches for names, and an
/// over-eager strip can only cause a false pass on a line that holds no code.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_block = false;
    for line in src.lines() {
        let mut rest = line;
        loop {
            if in_block {
                match rest.find("*/") {
                    Some(end) => {
                        in_block = false;
                        rest = &rest[end + 2..];
                    }
                    None => break,
                }
            } else {
                let line_at = rest.find("//");
                let block_at = rest.find("/*");
                match (line_at, block_at) {
                    (Some(l), b) if b.is_none_or(|b| l < b) => {
                        out.push_str(&rest[..l]);
                        break;
                    }
                    (_, Some(b)) => {
                        out.push_str(&rest[..b]);
                        in_block = true;
                        rest = &rest[b + 2..];
                    }
                    _ => {
                        out.push_str(rest);
                        break;
                    }
                }
            }
        }
        out.push('\n');
    }
    out
}

#[test]
fn no_source_file_reaches_for_a_game() {
    let src = crate_root().join("src");
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    assert!(
        files.len() >= 5,
        "expected to scan the whole crate, found only {} file(s) — has the layout moved?",
        files.len()
    );

    let mut offenders = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let code = strip_comments(&text);
        for (n, line) in code.lines().enumerate() {
            for marker in FORBIDDEN_DEP_MARKERS {
                if line.contains(marker) {
                    offenders.push(format!("{}:{} — {}", relative(path), n + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "bevy_lights must stay game-agnostic, but {} line(s) reference a forbidden name.\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

#[test]
fn the_dependency_list_stays_closed() {
    let manifest =
        std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("bevy_lights must have a Cargo.toml");

    // Only the `[dependencies]` table. The dev-dependency (the full Bevy umbrella) is examples and
    // tests, not part of the library's graph.
    let deps =
        manifest.split("[dependencies]").nth(1).expect("bevy_lights must declare a [dependencies] table");
    let deps = deps.split("\n[").next().unwrap_or(deps);

    for line in deps.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // `.` is a separator too, so `serde.workspace = true` reads as `serde`.
        let name = line.split(['=', ' ', '.']).next().unwrap_or("");
        // Continuation lines of a multi-line value (feature arrays, closing brackets) are not crate
        // names; skip them by shape.
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        assert!(
            ALLOWED_DEPS.contains(&name),
            "bevy_lights declares `{name}`, which is not in its allowed set {ALLOWED_DEPS:?}. \
             Widening this is a design decision: add it to ALLOWED_DEPS in this test in the same \
             commit so the change is visible in review."
        );
        for marker in FORBIDDEN_DEP_MARKERS {
            assert!(
                !name.contains(marker),
                "bevy_lights declares `{name}`, which contains the forbidden marker `{marker}`."
            );
        }
    }
}

#[test]
fn the_pure_model_is_engine_free() {
    let mut files = Vec::new();
    rust_sources(&crate_root().join("src/model"), &mut files);
    assert!(!files.is_empty(), "expected src/model to hold the engine-free math; has the layout moved?");

    let mut offenders = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let code = strip_comments(&text);
        for (n, line) in code.lines().enumerate() {
            if line.contains("bevy") {
                offenders.push(format!("{}:{} — {}", relative(path), n + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "src/model/** must be plain math with no `bevy` in code, but {} line(s) name it.\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}
