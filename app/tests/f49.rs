//! F49 at the binary: a theme file's `art` is never read, whatever it holds, and the file loads. It replaces F32's
//! step 4: no theme file is refused for what its `art` names, and the same file loads the same on every system.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::{layout, page, wish};

const PICTURES: &str =
    "jellymoon-app: theme: art: pictures in a theme are not read in this version; the kit's own art is drawn\n";

/// `jellymoon-app layout` under the theme file, given thirty seconds: None if it was still going then. Its output
/// goes to files, so a program that waits is never one that waits on a full pipe.
fn within(dir: &Path, day: &str, theme: &Path) -> Option<Output> {
    fs::write(dir.join("page.json"), day).unwrap();
    let to = |name: &str| Stdio::from(fs::File::create(dir.join(name)).unwrap());
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jellymoon-app"));
    cmd.args(["layout", "--page"]).arg(dir.join("page.json")).arg("--theme").arg(theme).env_clear();
    if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
        cmd.env("SystemRoot", root); // a Windows program needs it to start
    }
    let mut child = cmd.stdin(Stdio::null()).stdout(to("out")).stderr(to("err")).spawn().unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            child.wait().unwrap();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Some(Output { status, stdout: fs::read(dir.join("out")).unwrap(), stderr: fs::read(dir.join("err")).unwrap() })
}

#[test]
fn art_in_any_shape_naming_anything_is_not_read_and_the_file_loads_with_the_kit_s_own_art() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let day = page(vec![wish("w1", "Login page", "working", &["form built"])], vec![]).to_string();
    let own = layout(&day, &[]);
    assert!(own.status.success() && !own.stdout.is_empty());
    let themes = [
        "[art]\nstar = \"../star.svg\"\n",
        "[art]\nstar = \"/etc/passwd\"\nhero = \"a/../../x.svg\"\nfuture = \"../x\"\n",
        "[art]\nstar = '..\\star.svg'\nhero = 'C:\\Windows\\x.svg'\nbadge = 'C:x.svg'\nbubble = '\\\\host\\share\\x.svg'\n",
        "[art]\nstar = \"javascript:alert(1)\"\nhero = \"<svg onload='alert(1)'><script>alert(1)</script></svg>\"\n",
        "[art]\nstar = \"\"\n",
        "[art]\n",
        "art = \"../x\"\n",
        "art = [\"../x\", \"/x\", 3]\n",
        "art = 7\n",
        "[art.star]\ndeep = \"../x\"\n[art.hero.deeper]\nstill = [\"/x\"]\n",
        "[[art]]\nstar = \"../x\"\n[[art]]\nstar = \"/x\"\n",
    ];
    for theme in themes {
        let file = dir.path().join("theme.toml");
        fs::write(&file, theme).unwrap();
        let out = layout(&day, &["--theme", file.to_str().unwrap()]);
        let said = String::from_utf8(out.stderr).unwrap();
        assert_eq!((out.status.code(), said.as_str()), (Some(0), PICTURES), "{theme}");
        assert_eq!(out.stdout, own.stdout, "the page is laid out with the kit's own art: {theme}");
    }
}

#[test]
fn nothing_an_art_names_is_opened_so_a_file_that_would_stall_or_fail_a_reader_does_neither() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let day = page(vec![wish("w1", "Login page", "working", &["form built"])], vec![]).to_string();
    let own = layout(&day, &[]).stdout;
    let theme = dir.path().join("theme.toml");
    let pictures = ["star.svg", "hero.svg", "pipe.svg", "gone.svg"];
    let named: Vec<String> =
        pictures.iter().map(|name| format!("{} = \"{name}\"\n", &name[..name.len() - 4])).collect();
    fs::write(&theme, format!("[art]\n{}", named.concat())).unwrap();
    // What fails a reader on every system: a folder of the name, text that is no picture, a name with nothing there.
    fs::create_dir(dir.path().join("star.svg")).unwrap();
    fs::write(dir.path().join("hero.svg"), [0xFF, 0xFE, 0x00]).unwrap();
    // What stalls one: a named pipe nobody writes to, which an open for reading waits on for ever. The standard
    // library makes none, so `mkfifo` is started as a program, as `crates/jellymoon/tests/f40/pipe.rs` does.
    match ["/usr/bin/mkfifo", "/bin/mkfifo"].iter().find(|at| cfg!(unix) && Path::new(at).is_file()) {
        Some(mkfifo) => assert!(Command::new(mkfifo).arg(dir.path().join("pipe.svg")).status().unwrap().success()),
        None => eprintln!(
            "left out: the named pipe. Windows has none as a file in a folder, and elsewhere mkfifo is in neither /usr/bin nor /bin"
        ),
    }
    let out = within(dir.path(), &day, &theme).expect("the app did not wait on a file the art names");
    assert_eq!((out.status.code(), String::from_utf8(out.stderr).unwrap().as_str()), (Some(0), PICTURES));
    assert_eq!(out.stdout, own);
}
