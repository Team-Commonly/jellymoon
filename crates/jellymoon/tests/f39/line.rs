//! The command line a run starts its harness with, as the tests of a run compare it (`world.rs`).

/// The schema of the harness's last word, as core passes it.
pub const SCHEMA: &str = r#"{"type":"object","properties":{"state":{"type":"string","enum":["done","asks"]},"question":{"type":"string"}},"required":["state"],"additionalProperties":false}"#;

/// The command line measured on Claude Code 2.1.322 and 2.1.323, and the one fixed text core adds to it. `<id>`
/// stands where the session id is, which is another for every run ([`World::args`]).
pub const ARGS: [&str; 22] = [
    "-p",
    "--restricted",
    "--strict-mcp-config",
    "--permission-prompts",
    "none",
    "--permission-mode",
    "acceptEdits",
    "--tools",
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "--session-id",
    "<id>",
    "--json-schema",
    SCHEMA,
    "--output-format",
    "stream-json",
    "--verbose",
    "--append-system-prompt",
    "You are working in a copy of a repo that was made for this one wish. You can read and edit files in this \
     folder only. You cannot run commands. Make the change the wish asks for, then stop with the state `done`. If \
     you cannot make it without an answer that only the person can give, stop with the state `asks` and one \
     question for them, in plain words.",
];

/// Whether a text is a session id as core makes one: a random UUID in small letters.
pub fn is_session(id: &str) -> bool {
    let shaped = |(at, byte): (usize, u8)| match at {
        8 | 13 | 18 | 23 => byte == b'-',
        14 => byte == b'4',
        19 => b"89ab".contains(&byte),
        _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
    };
    id.len() == 36 && id.bytes().enumerate().all(shaped)
}
