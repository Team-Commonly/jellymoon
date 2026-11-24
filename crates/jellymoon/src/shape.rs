//! `jellymoon shape`, and what `wish --shaped` adds to `wish` (F91, ADR 0025). The rules are core's
//! (`run/shape.rs`): here is the command line, and the one line said when no shape was used.

use std::path::{Path, PathBuf};

use jellymoon_core::{Error, Wished};
use lexopt::prelude::*;

use crate::Failure;
use crate::parse::{Command, found};

/// Said beside a wish that `--shaped` kept as typed. The wish is kept all the same: this is no failure.
const AS_TYPED: &str =
    "jellymoon: the wish is kept as typed: no sharper wish was kept with these words and this project";

/// `shape`: the words and the project, and what finds the project's folder as `wish --run` finds it.
pub fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut say, mut project, mut logs, mut skip) = (None, None, None, None, String::new());
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("say") => say = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("project") => project = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("logs") => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") => skip = parser.value().and_then(|v| v.string()).map_err(usage)?,
            other => return Err(usage(other.unexpected())),
        }
    }
    let (Some(say), Some(project)) = (say, project) else {
        return Err(Failure::Usage("shape takes --say and --project".to_string()));
    };
    let mut options = found(logs, &skip)?;
    options.day = date;
    Ok(Command::Shape(options, say, project))
}

/// `wish --shaped`: the wish core kept, said to be as typed when no shape was used.
pub fn wish(home: &Path, date: Option<&str>, say: &str, project: &str) -> Result<Wished, Error> {
    let (made, with_shape) = jellymoon_core::wish_shaped(home, date, say, project)?;
    if !with_shape {
        eprintln!("{AS_TYPED}");
    }
    Ok(made)
}
