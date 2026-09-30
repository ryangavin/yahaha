//! Checks the JSON examples in docs/app-api.md against the app API's Rust types: every
//! `{"type": ...}` object written out in the doc must parse as an `AppCmd` (or else an
//! `Event`, or the Launchkey surface's `Layer`) and serialize back to the same JSON, and the doc's example `AppState` must do
//! the same as an `AppState`.
//!
//! The doc is read at run time from the path given, never compiled in, so a docs-only
//! change can't change what the test binaries test (AGENTS.md, Checks). CI runs it on
//! every PR, docs-only ones included (the `api-doc` job):
//!
//! ```text
//! cargo run --no-default-features --example api_doc_check -- docs/app-api.md
//! ```
//!
//! Exits 0 when every example matches, 1 on a mismatch (each one printed), 2 on bad usage.

use std::process::ExitCode;

use serde_json::Value;
use yahaha::launchkey::Layer;
use yahaha::{AppCmd, AppState, Event};

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: api_doc_check <path to docs/app-api.md>");
        return ExitCode::from(2);
    };
    let doc = match std::fs::read_to_string(&path) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    match check(&doc) {
        Ok(found) => {
            println!(
                "{path}: {} commands, {} events and the example AppState match the Rust types",
                found.cmds, found.events
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for p in &problems {
                eprintln!("{path}: {p}");
            }
            eprintln!("{path}: {} example(s) don't match the Rust types", problems.len());
            ExitCode::FAILURE
        }
    }
}

/// How many examples of each kind matched.
#[derive(Debug)]
struct Found {
    cmds: usize,
    events: usize,
}

/// Checks every example in `doc`. Returns what matched, or every problem found.
fn check(doc: &str) -> Result<Found, Vec<String>> {
    let mut problems = Vec::new();
    let mut found = Found { cmds: 0, events: 0 };

    for text in type_objects(doc) {
        let v: Value = serde_json::from_str(text).expect("type_objects only returns JSON");
        if let Ok(cmd) = serde_json::from_value::<AppCmd>(v.clone()) {
            match serde_json::to_value(&cmd) {
                Ok(out) if out == v => found.cmds += 1,
                Ok(out) => problems.push(format!("{text}: comes back from AppCmd as {out}")),
                Err(e) => problems.push(format!("{text}: AppCmd won't serialize: {e}")),
            }
        } else {
            match serde_json::from_value::<Event>(v.clone()) {
                Ok(ev) => match serde_json::to_value(&ev) {
                    Ok(out) if out == v => found.events += 1,
                    Ok(out) => problems.push(format!("{text}: comes back from Event as {out}")),
                    Err(e) => problems.push(format!("{text}: Event won't serialize: {e}")),
                },
                // The Launchkey surface's layer (`surface.layer`) is `type`-tagged too.
                Err(_) if serde_json::from_value::<Layer>(v.clone()).is_ok_and(|l| serde_json::to_value(l).is_ok_and(|out| out == v)) => {}
                Err(e) => {
                    // Report why it isn't a command, which is what almost every example is.
                    let why = serde_json::from_value::<AppCmd>(v).unwrap_err();
                    problems.push(format!("{text}: neither AppCmd ({why}) nor Event ({e}) nor a surface layer"));
                }
            }
        }
    }
    // Guards against the scan silently finding nothing (a changed doc layout).
    if found.cmds < 10 {
        problems.push(format!("only {} command examples found; expected at least 10", found.cmds));
    }

    if let Err(p) = check_example_state(doc) {
        problems.push(p);
    }

    if problems.is_empty() { Ok(found) } else { Err(problems) }
}

/// The first ```json block after the "## Example `AppState`" heading must round-trip
/// through `AppState`.
fn check_example_state(doc: &str) -> Result<(), String> {
    const HEADING: &str = "## Example `AppState`";
    let start = doc.find(HEADING).ok_or(format!("no \"{HEADING}\" section"))?;
    let body = &doc[start..];
    let s = body.find("```json\n").ok_or(format!("no ```json block after \"{HEADING}\""))? + "```json\n".len();
    let e = s + body[s..].find("```").ok_or(format!("unclosed ```json block after \"{HEADING}\""))?;
    let v: Value = serde_json::from_str(&body[s..e]).map_err(|e| format!("example AppState isn't JSON: {e}"))?;
    let st: AppState = serde_json::from_value(v.clone()).map_err(|e| format!("example AppState doesn't parse: {e}"))?;
    let out = serde_json::to_value(&st).map_err(|e| format!("example AppState won't serialize: {e}"))?;
    if out != v {
        return Err("example AppState doesn't come back out of AppState unchanged".to_string());
    }
    Ok(())
}

/// Every `{"type": ...}` object written out in the doc (inline examples and the example
/// state's actions), as its text. Objects that aren't JSON are skipped: the doc also
/// writes the general form, `{"type": "<camelCaseVariant>", ...fields}`.
fn type_objects(doc: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = doc.as_bytes();
    let mut i = 0;
    while let Some(off) = doc[i..].find('{') {
        let s = i + off;
        if doc[s + 1..].trim_start().starts_with("\"type\"") {
            let mut depth = 0;
            let mut j = s;
            while j < b.len() {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let text = &doc[s..=j.min(b.len() - 1)];
            if serde_json::from_str::<Value>(text).is_ok() {
                out.push(text);
            }
        }
        i = s + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = include_str!("../tests/fixtures/state.json");

    /// A doc with ten good commands and the fixture state as its example `AppState`.
    fn doc_with(extra: &str) -> String {
        let cmds = r#"`{"type":"startStop"}`, "#.repeat(10);
        format!("# API\n\n{cmds}\n{extra}\n\n## Example `AppState`\n\n```json\n{STATE}\n```\n")
    }

    #[test]
    fn good_examples_pass() {
        let general_form = r#"`{"type": "<camelCaseVariant>", ...fields}`"#;
        let base = check(&doc_with("")).unwrap().cmds;
        // The ten inline commands plus the actions inside the state; the general form,
        // which isn't JSON, is skipped rather than failing.
        assert!(base > 10);
        assert_eq!(check(&doc_with(general_form)).unwrap().cmds, base);
        let one_more = check(&doc_with(r#"`{"type":"main","index":1}`"#)).unwrap().cmds;
        assert_eq!(one_more, base + 1);
    }

    #[test]
    fn a_command_with_a_wrong_field_fails() {
        let problems = check(&doc_with(r#"`{"type":"main","indx":1}`"#)).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("indx"), "{problems:?}");
    }

    #[test]
    fn an_unknown_command_fails() {
        let problems = check(&doc_with(r#"`{"type":"noSuchCommand"}`"#)).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
    }

    #[test]
    fn a_broken_example_state_fails() {
        let doc = doc_with("").replacen("\"version\"", "\"versionX\"", 1);
        let problems = check(&doc).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("example AppState")), "{problems:?}");
    }

    #[test]
    fn a_doc_with_no_examples_fails() {
        assert!(check("# API\n").is_err());
    }
}
