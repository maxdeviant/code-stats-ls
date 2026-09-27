//! Support for recording XP from a Claude Code `PostToolUse` hook.
//!
//! Claude Code writes files directly to disk, so an editor-hosted language
//! server never sees (most of) its edits. Instead, the hook tells us exactly
//! which file was written and what text was inserted.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Local;
use serde::Deserialize;
use tokio::io::AsyncReadExt;

use crate::cache::PulseCache;
use crate::config::Config;
use crate::languages::language_for_extension;
use crate::pulse::{Pulse, PulseSender, PulseXp};

/// The maximum number of pulses to send in a single hook invocation.
///
/// Claude Code waits on the hook, so we don't want to spend too long draining
/// a large backlog of cached pulses in one go.
const MAX_PULSES_PER_INVOCATION: usize = 5;

#[derive(Debug, Deserialize)]
struct HookInput {
    tool_name: String,
    tool_input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct EditInput {
    file_path: String,
    new_string: String,
}

#[derive(Debug, Deserialize)]
struct WriteInput {
    file_path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MultiEditInput {
    file_path: String,
    edits: Vec<MultiEditEntry>,
}

#[derive(Debug, Deserialize)]
struct MultiEditEntry {
    new_string: String,
}

/// Returns the XP gained from the given tool use, if any.
///
/// XP is awarded as one point per character written, mirroring what it would
/// have cost to type it out.
fn xp_for_hook_input(input: HookInput) -> Result<Option<PulseXp>> {
    let (file_path, xp) = match input.tool_name.as_str() {
        "Edit" => {
            let edit: EditInput = serde_json::from_value(input.tool_input)?;
            (edit.file_path, edit.new_string.chars().count())
        }
        "Write" => {
            let write: WriteInput = serde_json::from_value(input.tool_input)?;
            (write.file_path, write.content.chars().count())
        }
        "MultiEdit" => {
            let multi_edit: MultiEditInput = serde_json::from_value(input.tool_input)?;
            let xp = multi_edit
                .edits
                .iter()
                .map(|edit| edit.new_string.chars().count())
                .sum();
            (multi_edit.file_path, xp)
        }
        _ => return Ok(None),
    };

    let Some(language) = Path::new(&file_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(language_for_extension)
    else {
        return Ok(None);
    };

    let xp = u32::try_from(xp).unwrap_or(u32::MAX);
    if xp == 0 {
        return Ok(None);
    }

    Ok(Some(PulseXp {
        language: language.to_string(),
        xp,
    }))
}

fn user_agent() -> String {
    format!(
        "{name}/{version} (Claude Code)",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Records XP from the Claude Code hook input provided on stdin.
pub async fn run(config: Config) -> Result<()> {
    let mut stdin = String::new();
    tokio::io::stdin()
        .read_to_string(&mut stdin)
        .await
        .context("failed to read hook input from stdin")?;

    let input: HookInput = serde_json::from_str(&stdin).context("failed to parse hook input")?;

    let pulse_cache = PulseCache::new()?;

    if let Some(pulse_xp) = xp_for_hook_input(input)? {
        let pulse = Pulse {
            coded_at: Local::now().to_rfc3339(),
            xps: vec![pulse_xp],
        };

        // Always go through the cache so that the pulse isn't lost if sending fails.
        pulse_cache.save(&pulse)?;
    }

    let sender = PulseSender::new(config, Duration::from_secs(3));
    let user_agent = user_agent();

    let mut pulses = pulse_cache.take(MAX_PULSES_PER_INVOCATION)?.into_iter();
    while let Some(pulse) = pulses.next() {
        if let Err(err) = sender.send(&pulse, &user_agent).await {
            // Put back anything we didn't send and try again next time. Being
            // offline shouldn't surface as a hook failure in Claude Code.
            for pulse in std::iter::once(pulse).chain(pulses) {
                pulse_cache.save(&pulse)?;
            }

            eprintln!("Error sending XP pulse (cached for later): {err}");
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn xp_for(value: serde_json::Value) -> Option<PulseXp> {
        xp_for_hook_input(serde_json::from_value(value).unwrap()).unwrap()
    }

    #[test]
    fn test_edit() {
        let xp = xp_for(json!({
            "session_id": "abc",
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/tmp/project/src/main.rs",
                "old_string": "fn main() {}",
                "new_string": "fn main() { println!(\"hi\"); }",
            },
            "tool_response": {},
        }))
        .unwrap();

        assert_eq!(xp.language, "Rust");
        assert_eq!(xp.xp, 29);
    }

    #[test]
    fn test_write() {
        let xp = xp_for(json!({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "/tmp/project/src/app.gleam",
                "content": "pub fn main() { Nil }\n",
            },
        }))
        .unwrap();

        assert_eq!(xp.language, "Gleam");
        assert_eq!(xp.xp, 22);
    }

    #[test]
    fn test_multi_edit() {
        let xp = xp_for(json!({
            "tool_name": "MultiEdit",
            "tool_input": {
                "file_path": "/tmp/project/index.ts",
                "edits": [
                    { "old_string": "a", "new_string": "abc" },
                    { "old_string": "b", "new_string": "de" },
                ],
            },
        }))
        .unwrap();

        assert_eq!(xp.language, "TypeScript");
        assert_eq!(xp.xp, 5);
    }

    #[test]
    fn test_counts_characters_not_bytes() {
        let xp = xp_for(json!({
            "tool_name": "Write",
            "tool_input": { "file_path": "README.md", "content": "héllo 👋" },
        }))
        .unwrap();

        assert_eq!(xp.xp, 7);
    }

    #[test]
    fn test_ignored_tool_uses() {
        // Unknown language.
        assert!(xp_for(json!({
            "tool_name": "Write",
            "tool_input": { "file_path": "Makefile", "content": "all:" },
        }))
        .is_none());

        // Deletion.
        assert!(xp_for(json!({
            "tool_name": "Edit",
            "tool_input": { "file_path": "main.rs", "old_string": "x", "new_string": "" },
        }))
        .is_none());

        // Other tools.
        assert!(xp_for(json!({
            "tool_name": "Bash",
            "tool_input": { "command": "ls" },
        }))
        .is_none());
    }
}
