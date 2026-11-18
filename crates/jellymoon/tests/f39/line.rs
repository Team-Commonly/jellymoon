//! The command line a run starts its harness with, as the tests of a run compare it (`world.rs`).

/// The schema of the harness's last word, as core passes it: since F87 with a result of four kinds of pieces.
pub const SCHEMA: &str = r#"{"type":"object","properties":{"state":{"type":"string","enum":["done","asks"]},"question":{"type":"string"},"result":{"type":"object","properties":{"pieces":{"type":"array","maxItems":12,"items":{"type":"object","properties":{"type":{"type":"string","enum":["did","change","decided","unsure"]},"text":{"type":"string","maxLength":280},"file":{"type":"string"},"before":{"type":"array","maxItems":3,"items":{"type":"string","maxLength":100}},"after":{"type":"array","maxItems":3,"items":{"type":"string","maxLength":100}}},"required":["type"],"additionalProperties":false}}},"required":["pieces"],"additionalProperties":false}},"required":["state"],"additionalProperties":false}"#;

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
    "You are working in a copy of a repo that was made for this one wish. You can read and edit files in this folder \
     only. You cannot run commands. Make the change the wish asks for, then stop with the state `done` and a `result` \
     for the person who made the wish. If you cannot make it without an answer that only the person can give, stop \
     with the state `asks` and one question for them, in plain words, and no `result`. A `result` is a short list of \
     pieces. First one `did`: one or two plain sentences that a person who does not program can read, saying what \
     changed for them, not how. Then, for the changed files that matter most, at most two `change` pieces: `file` is \
     the file's path in this folder, `before` holds only lines that are gone or read differently now, and `after` only \
     lines that are new or read differently now, at most three on each side. Quote only lines that changed: never \
     repeat an unchanged line on either side for context. For lines that were only added, `before` is empty; for lines \
     that were only removed, `after` is empty. Each line is whole and copied exactly as it is in the file, character \
     for character, with nothing left out and no `...`, and under 100 characters: a piece with a line that is not \
     exact, or did not change, is dropped. Then a `decided` only for a choice the person might have made differently, \
     and an `unsure` only for something the person should look at, each one short sentence, at most three of each. You \
     ran nothing, so never say that anything was run or tested.",
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
