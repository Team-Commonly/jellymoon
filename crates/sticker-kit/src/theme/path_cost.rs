//! What a path's `d` costs to draw, counted as the renderer reads it: command by command, with the pen followed
//! through the relative ones. Two things make a path slow, and both were measured (`README.md`). One is how many
//! numbers it holds, and a number of an arc or of a `t` costs more than a line's, so each kind of command has its
//! own count. The other is how far a curve reaches: an arc of radius 4096 on a picture 8 wide took 60 times as long
//! as a line, and no count that still lets an ordinary picture in could hold that. So a path stays near its picture.
//!
//! What this cannot read the way the renderer does, it refuses: an arc's flags run into the number after them, a
//! number where no command takes one, a character that is neither.

/// How far from the picture's corner a path may name a point or take its pen, and the largest radius of an arc:
/// twice the largest side. A group may still move the path anywhere: what costs is a curve's size, not its place.
pub const MAX_ART_REACH: f32 = 512.0;

/// Each path command: its letter, how many numbers one takes, and what each of those numbers counts as against
/// `MAX_ART_NUMBERS`. An arc is drawn as several curves and a `t` is a whole curve for two numbers: theirs count
/// two. That put the worst picture of each kind inside [`MAX_ART_REACH`] within about twice the worst of lines
/// (`README.md` has the times).
const COMMANDS: [(char, usize, usize); 10] = [
    ('m', 2, 1),
    ('l', 2, 1),
    ('h', 1, 1),
    ('v', 1, 1),
    ('z', 0, 0),
    ('q', 4, 1),
    ('t', 2, 2),
    ('c', 6, 1),
    ('s', 4, 1),
    ('a', 7, 2),
];

/// What the numbers of a `d` count as together, or why it is refused. `d` is as the file wrote it: a capital
/// letter's numbers are places, a small one's are steps from where the pen is.
pub fn path_cost(d: &str) -> Result<usize, String> {
    let (mut pen, mut to, mut start, mut cost) = ([0.0_f32; 2], [0.0_f32; 2], [0.0_f32; 2], 0);
    // The command being read, whether it steps, and how many numbers it has been given.
    let (mut command, mut steps, mut given) = (None, false, 0);
    // The last handle of the curve before, and its kind (`q` or `c`): an `s` or a `t` starts from its mirror.
    let (mut handle, mut named) = (None, [0.0_f32; 2]);
    let mut rest = d;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        let Some(next) = rest.chars().next() else {
            return Ok(cost);
        };
        if let Some(found) = COMMANDS.iter().find(|(letter, ..)| *letter == next.to_ascii_lowercase()) {
            (command, steps, given, rest) = (Some(*found), next.is_ascii_lowercase(), 0, &rest[1..]);
            (pen, handle) = if found.0 == 'z' { (start, None) } else { (pen, handle) };
            continue;
        }
        let (text, after) = number(rest).ok_or_else(|| format!("its path holds `{next}` where a number is read"))?;
        let Some((letter, takes, counts)) = command.filter(|(_, takes, _)| *takes > 0) else {
            return Err("its path holds a number that no command takes".into());
        };
        let (nth, value) = (given % takes, text.parse::<f32>().unwrap_or(f32::INFINITY));
        if nth == 0 {
            to = pen;
            let mirror = mirrored(letter, pen, handle);
            if mirror.iter().any(|side| side.abs() > MAX_ART_REACH) {
                return Err(far());
            }
            handle = if letter == 't' { Some(('q', mirror)) } else { handle };
        }
        // Which of the pen's two sides a number is, when it is a place: an arc's first five are not.
        let side = match (letter, nth) {
            ('a', 0 | 1) if value.abs() > MAX_ART_REACH => return Err(far()),
            ('a', 3 | 4) if text != "0" && text != "1" => {
                return Err("an arc's two flags are each `0` or `1`, apart from the numbers beside them".into());
            }
            ('a', 0..=4) => None,
            ('h', _) => Some(0),
            ('v', _) => Some(1),
            ('a', _) => Some((nth - 5) % 2),
            _ => Some(nth % 2),
        };
        if let Some(side) = side {
            let place = if steps { pen[side] + value } else { value };
            if place.abs() > MAX_ART_REACH {
                return Err(far());
            }
            // The last pair of a command is where the pen goes; the pairs before it are a curve's handles.
            match takes.saturating_sub(nth) {
                1 | 2 => to[side] = place,
                3 | 4 => named[side] = place,
                _ => {}
            }
        }
        (given, cost, rest) = (given + 1, cost + counts, after);
        if nth + 1 == takes {
            pen = to;
            start = if letter == 'm' && given == 2 { pen } else { start };
            handle = left(letter, named, handle);
        }
    }
}

/// The handle a finished command leaves for an `s` or a `t` after it, with its kind: the one a `q`, `c` or `s`
/// wrote last, the one a `t` mirrored, and none after anything else.
fn left(letter: char, named: [f32; 2], mirror: Option<(char, [f32; 2])>) -> Option<(char, [f32; 2])> {
    match letter {
        'q' => Some(('q', named)),
        'c' | 's' => Some(('c', named)),
        't' => mirror,
        _ => None,
    }
}

/// The handle an `s` or a `t` does not write: the mirror, through the pen, of the last handle of a curve of its
/// kind just before it, or the pen itself after anything else. A row of `t` between two far points takes it
/// further out at each one, so it is held to the reach as a written handle is. For any other letter, the pen.
fn mirrored(letter: char, pen: [f32; 2], handle: Option<(char, [f32; 2])>) -> [f32; 2] {
    let kind = match letter {
        't' => 'q',
        's' => 'c',
        _ => return pen,
    };
    match handle {
        Some((of, at)) if of == kind => [2.0 * pen[0] - at[0], 2.0 * pen[1] - at[1]],
        _ => pen,
    }
}

fn far() -> String {
    format!("its path goes more than {MAX_ART_REACH} from the picture's corner, or has an arc of a larger radius")
}

/// The number `text` starts with, as written, and what follows it: a sign or none, then digits with one `.` at
/// most. `None` when it starts with no number.
fn number(text: &str) -> Option<(&str, &str)> {
    let digits = |from: usize| text[from..].bytes().take_while(u8::is_ascii_digit).count();
    let sign = usize::from(text.starts_with(['-', '+']));
    let whole = digits(sign);
    let decimals = if text[sign + whole..].starts_with('.') { 1 + digits(sign + whole + 1) } else { 0 };
    let len = sign + whole + decimals;
    (whole + decimals > usize::from(decimals > 0)).then(|| text.split_at(len))
}
