# 0019 · A wish said aloud: a held key, an ear on the machine, a jellyfish at the screen's edge

Status: Accepted by the coordinator, 2026-11-13, as written by a designer on the owner's direction. Nothing here was
built or run: every claim about a system was read, and its source is in "Sources" as [S n]. What was not found
there says "not confirmed". Decisions 5 and 6 are taken as they stand, so F55 and F56 can be built now. Decisions 1,
2 and 4 hold until their spike says otherwise ("Three things to try first"): F58 waits on spike 2, F59 on spike 1,
F57 on spike 3. A Developer ID (decision 3) is the owner's to decide and nothing waits on it.

## Why

The owner's direction: "a new way to vibe code fun without using tui or gui for all kinds of ADE", "as easy and
fun as installing minecraft and building a wooden house with friends", "it should feel like a video game", and on
stack: "smart in stack choice we want to build faster and stable and maintainable software for real".

The loop is wish, watch, answer, see, keep or toss ([ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md)). A wish
is typed today, in the page's entry or with `jellymoon wish`. This note decides the next step of the build order
(ADR 0017, "What comes after", 2): a wish said aloud from anywhere, with the app not in front, and a small window
at the screen's edge that shows what was heard and what the runs are doing.

## What binds the design

- **Core never depends on a platform**, and the shell decides nothing (ADR 0011, ADR 0012).
- **Only the reading call and a run send anything out.** So a voice is turned into words on the machine.
- **`unsafe` is forbidden in every crate** (`Cargo.toml`, line 29). Every Apple and Windows interface below is
  reached through `unsafe` somewhere. Where that somewhere is, is a decision of this note.
- **Rust only** (ADR 0012). ADR 0011, section 4, planned "a small Swift helper" for speech. It was written before
  the restart, and is changed here.
- **The release is signed ad hoc** (`app/macos/bundle.sh`, lines 58 and 59), with no Apple identity. That decides
  more of this note than any other fact: see decision 3.

## Decisions

### 1. The key: one system-wide hot key, registered with `global-hotkey`, and no permission

The app registers one key combination with the system. Held, the ear listens. Let go, it stops.

| Option | Needs from the person | Down and up | Cost |
|---|---|---|---|
| GPUI at the pinned commit | | | It has none: no hot key, event tap or global monitor in `gpui`, `gpui_macos`, `gpui_windows` [S1] |
| **Carbon `RegisterEventHotKey`, by the `global-hotkey` crate** | nothing: no permission is asked for or checked [S2] | both: the crate sends `Pressed` and `Released` [S2] | One combination with a real key in it; a modifier alone (hold Fn, hold right Option) cannot be one |
| `CGEventTap` | Input Monitoring, in Privacy & Security [S3][S4] | both, for any key | A permission that reads every key the person types, to learn about one |
| `NSEvent` global monitor | Accessibility, for key events [S5] | both | The same, and it cannot keep the key from the app in front [S5] |
| Windows `RegisterHotKey` | nothing | down only: it posts `WM_HOTKEY` and has no message for the release [S6] | The crate asks the key's state every 50 ms after a press and sends `Released` when it is up [S7] |
| Windows low-level hook | nothing | both [S8] | Sees every key; removed silently by the system when slow [S8] |

**Chosen: `global-hotkey`** in the shell, behind its `window` feature, on the Mac and on Windows. It is safe Rust
to call, it is MIT or Apache-2.0 [S9], and ADR 0011's test registered such a crate inside GPUI's loop. On the Mac
it needs an event loop on the main thread, which GPUI runs [S9]. It is not built on Linux, where it would bring X11
packages for a system voice does not support.

**Why not a tap.** A tap would let the key be one modifier, which is nicer to hold. It costs a permission whose
name is "monitor your keyboard" [S4], asked of a person who was promised Minecraft. And under ad hoc signing the
grant does not carry over an update (decision 3).

**The default is Control+Shift+Space.** Command+Space, Control+Space, Control+Option+Space, Option+Command+Space,
Control+Command+Space and Shift+Command+Space are the system's [S10]; Control+Space switches input sources, which a
person who types Chinese uses all day. The key is one line of a settings file (decision 6) and the owner can
overrule the default at no cost.

**First run.** Nothing is asked for the key. When the system refuses the registration, the jellyfish says so in
core's fixed words and names the settings file. Typing a wish always works.

**What would change this:** the key not reporting its release inside GPUI's loop on a real build (spike 2). Then
the Mac adapter becomes a tap, and the first run gains the Input Monitoring step.

### 2. The ear: Apple's Speech framework, held to the device, in one Rust crate

| Option | Runs on | Leaves the machine | Added to the download | New dependency | Languages | Reached from Rust |
|---|---|---|---|---|---|---|
| **`SFSpeechRecognizer`, on-device required** | macOS 10.15 and later [S11] | nothing, when `requiresOnDeviceRecognition` is set and the recognizer supports it [S12] | nothing: the models are the system's | `objc2-speech`, `objc2-avf-audio`, bindings only [S13] | one language per recognizer [S11]; which have on-device support is asked at run time. English and Chinese on-device: not confirmed | yes: both are Objective-C interfaces, and `objc2` 0.6 is in the lock file already |
| `SpeechAnalyzer` with `SpeechTranscriber` | macOS 26 only [S14] | nothing [S15] | nothing; the system fetches a language's model [S14] | a Swift helper | read from `supportedLocales` at run time [S16]; Chinese: not confirmed | no: Swift only, a `final actor` with no Objective-C form [S14] |
| Whisper (`whisper.cpp` by `whisper-rs`) | Mac and Windows | nothing | 142 MiB (`base`) to 466 MiB (`small`) of model, bundled or fetched once [S17] | `whisper-rs` (a C++ build in CI on three systems) and an audio crate such as `cpal` [S18][S19] | many, in one model | yes |
| System dictation | Mac | the person's own setting decides | nothing | none | the system's | no interface an app can call for words: not confirmed |
| Windows `Windows.Media.SpeechRecognition` | Windows | the audio: free dictation "is performed by a remote web service" [S20] | nothing | | | not usable under the repo's rule |

**Chosen for the Mac now: `SFSpeechRecognizer` with on-device recognition required.** The ear sets
`requiresOnDeviceRecognition`, and first asks `supportsOnDeviceRecognition`. Apple honours the first only when the
second is true [S12]. So when it is false the ear starts nothing and says so: it never falls back to the network.

Why this one: it adds nothing to the download, it runs on every macOS the installer accepts (`install.sh`,
line 19: 11.0), and it keeps the repo Rust only. Apple says of it that "on-device requests won't be as accurate"
[S12], and it stops a task after one minute [S11], which no wish reaches (a wish is 200 characters at most).
`SpeechAnalyzer` is the better recognizer and Apple's future, but it would mean a Swift helper, a second toolchain
in the release, and macOS 26 only. Whisper is the one option that is the same code on both systems and can be
tested in CI with a made-up sound file; it costs a model of hundreds of megabytes before the first wish and a
C++ build on three systems. Latency for a five-second wish was measured for none of them: not confirmed.

**Where the seam is.** A new workspace crate, `crates/ear`, package `ear`. It knows nothing of Jellymoon, as the
kit does not. Its whole surface is safe Rust and plain data:

```rust
pub enum Said { Partial(String), Final(String), Cannot(Cannot) }
pub enum Cannot { NoEar, NoMicrophone, MicrophoneRefused, SpeechRefused, NotOnDevice, NoSuchLanguage, NotBundled, Failed }
impl Ear {
    pub fn open(language: &str) -> Result<Ear, Cannot>;
    pub fn listen(&mut self) -> std::sync::mpsc::Receiver<Said>;
    pub fn stop(&mut self);
}
```

One file per system sits behind it. `mac.rs` is the only file in the workspace where `unsafe` may be written: the
crate keeps its own `[lints]` table, denies `unsafe_code` at its root and allows it on that one module, and
`cargo xtask check` refuses any other crate that leaves the workspace's lints. On every other system `open`
returns `NoEar`, so the shell builds everywhere and Windows is one more file. Core and the `jellymoon` binary may
never list `ear`, `objc2*` or `global-hotkey` (`entropy.toml`, `[forbidden]`). The app is the only crate that
joins the ear and core, as it joins the kit and core.

Not a helper program. ADR 0014's table called F30 "a helper prints the words". In one process the system's
prompts are the app's own, with the bundle's own usage strings. Whether a child program is given the app's grant
is not confirmed, and it is one risk fewer not to need it.

**Windows, later:** Whisper behind the same surface, since Windows' own free dictation sends audio out [S20]. That
is its own note when Windows is taken.

**What would change this:** `supportsOnDeviceRecognition` false for English or for Mandarin on the owner's Mac, or
words so wrong that wishes are taken back more often than made (spike 1). Then the Mac gets Whisper too, behind
the same surface, and the cost is the model.

### 3. The microphone: two usage strings, two questions on the first hold, and a grant that an update loses

- **The bundle's `Info.plist` gains two strings** (`app/macos/bundle.sh`). `NSMicrophoneUsageDescription` is
  required of an app that uses the microphone [S21]. `NSSpeechRecognitionUsageDescription` is required whether or
  not recognition is on-device, and without it the app "will crash when it calls requestAuthorization" [S15].
  - Microphone: "Jellymoon listens only while you hold the wish key. The sound never leaves this Mac."
  - Speech: "Jellymoon turns what you say into words on this Mac. Nothing you say is sent to Apple."
- **No strings, no asking.** `jellymoon-app` started outside a bundle (a developer's build, a test) has no
  `Info.plist`. The shell reads the bundle's `Info.plist` beside its own program, as a file, before the ear is
  opened; with either string missing the ear is never opened and the jellyfish says `NotBundled`'s words. So no
  test and no terminal can reach the crash.
- **When the person is asked.** At the first hold of the key, never at launch. The jellyfish first says, in
  fixed words, that macOS will ask twice and that the sound stays on the Mac. Then the system asks for speech
  recognition and for the microphone. The system remembers each answer [S15].
- **Refused.** The jellyfish says which of the two is off and where to turn it on (System Settings, Privacy &
  Security), once per hold. Nothing else changes: the page, the entry and every typed wish work as before.
- **What ad hoc signing costs.** macOS records an app's designated requirement with a grant, and "ad hoc signed
  code ... has a DR but it's tied to that specific version of the code" [S22]. Apple's engineer: macOS "is unable
  to tell that version N+1 of your app is the 'same code' as version N" [S23]. So after every update the person
  is asked again, or finds a grant that no longer works. For a microphone that is two clicks. For Input
  Monitoring or Accessibility it is a visit to Settings each time, which is the second reason decision 1 takes no
  such permission. The cure is a Developer ID signature [S23]. That costs money and is the owner's alone to
  decide; this note asks for the decision and does not wait on it.
- **No entitlement is needed today**, since the bundle is not signed with the hardened runtime. A notarised
  release would need the audio-input entitlement: not confirmed here, to be read when signing changes.

### 4. The jellyfish: a GPUI pop-up window, not click-through

GPUI at the pinned commit can open the window [S24]:

| Wanted | GPUI at the pin |
|---|---|
| Above other windows | `WindowKind::PopUp`: an `NSPanel` at the pop-up level, 101 |
| Never takes the keys | the panel is non-activating, and `focus: false` orders it front without making it key |
| On every space, over a full-screen app | `CanJoinAllSpaces` and `FullScreenAuxiliary` are set for `PopUp` |
| See-through around the art | `WindowBackgroundAppearance::Transparent` |
| A chosen screen | `display_id`; the app can list the displays and the primary one |
| The app lives with the page closed | `QuitMode::Explicit`, and `on_reopen` for a click on the Dock icon |
| Click-through | **no**: nothing sets `ignoresMouseEvents` |
| Windows | `PopUp` is a topmost tool window, shown without activating; click-through: no |

**Chosen:** the jellyfish is one small `PopUp` window in the lower right corner of the primary display, inside the
room the Dock and the menu bar leave. While it has words to say, a second `PopUp`, the bubble, is opened beside it
and removed when the words are done; no window is resized or moved. Closing the page's window no longer quits the
app (`app/src/window.rs`, line 53, quits today): the jellyfish stays, the Dock icon opens the page again, and Quit
ends both.

**It is not click-through, and that is on purpose at first.** A tap on the jellyfish opens the page. That is the
one thing it does with a tap, and it gives the person a way to the page with no Dock. The bubble takes no tap. It
covers a corner of the screen only while it speaks. Making either click-through needs one `unsafe` call on the
window's raw handle, or a change in GPUI; neither is taken.

Not confirmed, and only a person at a screen can see it: that opening a `PopUp` never brings the app to the front,
whether the system draws a shadow or a frame round a see-through panel, and whether the see-through part takes
clicks (spike 3).

**What it shows.** Core says which of these, and every word but the person's own and a run's question:

| State | When | The jellyfish | The bubble |
|---|---|---|---|
| `quiet` | nothing to say | floats | none |
| `listening` | the key is held | leans in | the words so far, as they come |
| `heard` | the key was let go and words were heard | holds them up | the words, the project's name, and a ring that empties in 4 seconds |
| `working` | a made wish's run is going | swims | none; a count when more than one goes |
| `asks` | a run asks | waves | the question, as the run's own words |
| `waits` | work waits to be kept or tossed | glows | none |
| `cannot` | something was refused | droops | core's fixed sentence, for 6 seconds |

When more than one holds, the first in this order wins: `listening`, `heard`, `cannot`, `asks`, `working`,
`waits`, `quiet`.

### 5. Heard words pass the door, are shown for four seconds, and then start

**The same door.** What the ear returns is untrusted text, like anything typed. The final words go through the
door a typed wish goes through (`still.rs`, [wish.md](../development/wish.md)): one line, nothing unseen,
credential shapes replaced, cleaned until nothing changes, 200 characters, never cut. The words so far, shown
while the key is held, go through the same cleaning and are only ever drawn. A heard wish is then made by the call
the entry uses (`wish_offered`), and its run is started by `run_apart`, with the installed command. Nothing about a
made wish, a run, a copy, a keep or a toss changes.

**Start, with a moment to take it back.** The other choice is a yes before every run.

- For a yes first: an agent is about to act on a repo on words a machine heard. A wrong word can change the wish.
- For starting: a run cannot hurt the person's checkout. It works in its own copy, runs nothing, and lands
  nothing until the person looks and keeps (ADR 0014, ADR 0017). What a misheard wish costs is one run of the
  person's plan, one of the day's eight made wishes, and a toss. A yes before every wish doubles the gestures of
  the one thing the app is for, and nobody asks twice in a game.

So: the heard words are shown for 4 seconds with the project's name, and then the wish is made and its run
starts. **Until then nothing is kept.** A heard wish is not a made wish, which matters because a made wish cannot
be taken back (`wish.md`, "Not covered yet"). One gesture takes it back: pressing the key again. Held, it listens
anew, so saying it again is the correction. Tapped, the words are dropped. The window is one constant in core.

**Three more rules, all core's.**

- A hold under 300 ms is a slip of the hand: nothing is heard.
- Words that are empty once cleaned make no wish and no noise: the jellyfish goes back to what it showed.
- A hold ends by itself at 60 seconds, which is the recognizer's own limit [S11].

**Which project.** A run finds its repo by the wish's project: the one folder of that name that something was
typed in that day ([run.md](../development/run.md)). With the app not in front there is no chip to choose. Core
picks, from the projects it offers now (`wishable`): the project of the day's last made wish if it is still
offered; else the only one offered; else none. With none, nothing is kept and the jellyfish says to make the first
wish of the day on the page. The name is shown with the heard words for the whole four seconds, so a wrong repo is
seen before anything starts. Guessing the repo from the app in front would take the Accessibility permission, and
is not done.

**A run that asks.** The jellyfish shows the question once F51's `question` is on the wish, which it is today.
Answering it by voice (hold the key while `asks` is shown: the words are the answer, shown for the same four
seconds, then given to `run answer`) waits on F52 and F54.

### 6. What goes where, and how each is proved with no microphone, key or window

| Piece | Where | Proved at |
|---|---|---|
| A heard wish's states and every rule of decision 5; the jellyfish's state and its fixed sentences; the settings file `voice.toml` (the key, the language) and what it refuses | core, `hear.rs` | `jellymoon hear`: events in on stdin as JSON lines (`key` down and up, the ear's `Said`, `tick` with milliseconds), one line of JSON out per change of what the jellyfish shows. Time is data, so no test waits. With `--run` a made wish's run starts, as with `jellymoon wish --run` |
| A character with a mood, a bubble of text, a ring; where a corner window goes on a screen | the kit: an edge scene, with a pure layout | the kit's layout tests |
| Core's state turned into the kit's edge scene | `app/src/jelly.rs` | `jellymoon-app layout --jelly <file>`: the pieces of both windows, and where they stand on `--screen <W>x<H>` |
| Key and ear events carried to core, off the window's thread | `app/src/hearing.rs`, with no GPUI in it, as `showing.rs` has none | a made-up window that is a program of its own, fed a script of events |
| The two windows, the key's registration, the ear's channel | `app/src/window.rs`: wiring | GPUI's test platform on the Mac for the windows; a unit test for the options asked; a person's walk for the rest |
| The ear | `crates/ear` | its own binary, `ear`: what it refuses with no bundle, on a system with no ear, and for a language name that is not one. That it hears: a person's walk, on each release |

No sound file is in the repo. Every test below the ear works on text, typed by hand. If a later ear can be fed a
file (Whisper can), its fixture is speech made by a synthesizer, never a person's recording.

## What is not done

- **No wake word and no listening without the key.** The microphone is open only while the key is held.
- **No sound is kept**, in memory past the hold or on disk, and no words but a made wish's.
- **No voice on Windows or Linux yet.** The key and the jellyfish are built for Windows; the ear says `NoEar`.
- **No click-through, no dragging the jellyfish, no choice of corner or screen**, and it does not follow the
  screen the person works on.
- **No language switch by voice and no mixed-language tuning.** One language at a time, from `voice.toml`, by
  default the system's.
- **No keep or toss by voice.** Those stay taps on the page: they change the person's checkout.
- **No Developer ID.** Asked of the owner in decision 3.

## Risks left open

1. **A project needs a folder typed in that day.** A person who only ever uses Jellymoon types into no harness, and
   the reader leaves out a run's copy (ADR 0018, decision 7). So on their second day no project is offered, and
   neither the entry nor the key can make a wish. This is older than voice, and voice makes it the first thing a
   new person meets. It needs its own note before voice is announced.
2. **On-device Chinese** on the owner's Mac, and how good on-device English is. Not confirmed.
3. **The grant after an update**, under ad hoc signing: asked again, or silently dead. Not confirmed which.
4. **The default key** may be taken by another app. The first to register wins: not confirmed.
5. **A person's walk is owed on every release** for the ear, the key and the windows. Nothing in CI can hold a
   key or speak.
6. **The words of a wish reach the model**, as a typed wish's do. "The sound never leaves this Mac" is true of
   the sound, and the usage strings say only that.

## What changes in earlier notes

Each line is changed by the feature that makes it true, not before.

| It says today | It will say |
|---|---|
| ADR 0011, section 4: "reached through a small Swift helper when F30 needs it" | Reached from Rust, in `crates/ear`. Changed by F59 |
| ADR 0011, "Still open": "On-device speech as chosen needs macOS 26" | The ear needs macOS 10.15, below the installer's floor. Changed by F59 |
| ADR 0014, the table: "F30: a helper prints the words" | The words come from the ear in the app's process and enter core as events. Changed by F58 |
| ADR 0013, decision 2: only the kit and the shell see GPUI | Unchanged; and only the shell sees the ear and the key. Added by F58 and F59 |

## F30

F30's description is "Holding a key and saying a wish starts an agent on it". Its three steps say nothing of a key
or a voice, and all three hold today, from the command line and from the window:

| F30's step | Holds today, proved by |
|---|---|
| A wish made in the app appears on today's page at once, in the person's exact words, working | F37 (`crates/jellymoon/tests/f37.rs`) and F42 (`app/tests/f42.rs`, "words entered are kept by core and drawn at once") |
| A run is started with the person's harness and the star appears when it reports it finished | F39 (`crates/jellymoon/tests/f39.rs`), F41 and F42 (`app/tests/f42/run.rs`) |
| With no harness the wish stays and says why | F39 (`tests/f39/refused.rs`, "with no harness") and F42 (`f42/run.rs`, "a run that was not started") |

What does not hold is the description, and F30's own test, `crates/jellymoon/tests/f30.rs`, which does not exist.
The row is left as it is. F55 adds that file: the three steps, entered through `jellymoon hear`. F30 is set to pass
last, after F60 passes and a person has held the key on a real build, as ADR 0011 asks.

## The slices

| Feature | What it adds | Waits on | Waits for the gate |
|---|---|---|---|
| F55 | A heard wish's rules in core, at `jellymoon hear`; `voice.toml`; F30's test file | nothing | yes: a new way to start a run |
| F56 | The jellyfish as data: the kit's edge scene, and `jellymoon-app layout --jelly` | F55 | no |
| F57 | The two windows, and an app that lives with the page closed | F56 | no |
| F58 | The key: `global-hotkey`, and events carried to core by `hearing.rs` | F55, F57 | yes |
| F59 | `crates/ear`: the Mac's ear, the usage strings, `unsafe` in one file | nothing; spike 1 first | yes: the microphone |
| F60 | The first hold: the ear joined to the key, every refusal in core's words | F58, F59 | yes |
| F61 | A held key answers a run that asks | F52, F54, F60 | yes |

## Three things to try first, thrown away after

Each is a throwaway build outside the repo, run by a person at a Mac. None is a feature.

1. **The ear, in a bundle signed as a release is.** From Rust, with `objc2-speech` and `objc2-avf-audio`: does the
   system ask twice; is `supportsOnDeviceRecognition` true for `en-US` and `zh-CN`; how long from letting go to the
   final words of a five-second wish; how wrong are the words; and what does a second build, with a new signature,
   do to both grants. This decides decision 2 and blocks F59.
2. **The key, inside GPUI's loop at the pin.** With `global-hotkey` and Control+Shift+Space, another app in front:
   do the press and the release both arrive; is the combination kept from the app in front; what happens while an
   input method composes, and when another app holds the same key.
3. **The jellyfish's window.** A see-through `PopUp` with `focus: false`: does opening it bring the app forward or
   take the keys; is it over a full-screen app and on every space; does the system draw a shadow or a frame; does
   the see-through part take clicks; does the app live on when the page's window closes; a second display.

## New dependencies

| Package | For | Licence | Size | Rejected |
|---|---|---|---|---|
| `global-hotkey` 0.10 | the key, Mac and Windows | Apache-2.0 or MIT [S9] | not measured; on the Mac it adds `keyboard-types` and uses `objc2` and `objc2-app-kit`, which the lock has [S9] | a hand-written Carbon call (`unsafe` of our own for what the crate tests on three systems); a tap (decision 1) |
| `objc2-speech` 0.3 | `SFSpeechRecognizer` | Zlib, Apache-2.0 or MIT [S13] | not measured; bindings to a system framework | a Swift helper; Whisper (decision 2) |
| `objc2-avf-audio` 0.3 | the microphone's buffers | the same [S13] | the same | `cpal`: a second audio stack to hand buffers to Apple's recognizer |

Nothing is added to what a person downloads but the growth of the app's own program, which is not measured.

## Sources

Read on 2026-11-13 unless a line says otherwise. GPUI lines are of the Zed repository at the commit pinned in
`Cargo.toml`.

- [S1] GPUI at the pin: a search of `crates/gpui`, `crates/gpui_macos`, `crates/gpui_windows`, `crates/gpui_platform`
  and `crates/gpui_apple` for hot key, event tap and global monitor names finds nothing.
- [S2] `global-hotkey`, `src/platform_impl/macos/mod.rs`, branch `dev`:
  https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/macos/mod.rs . `RegisterEventHotKey`,
  `kEventHotKeyPressed` and `kEventHotKeyReleased`; a tap only for media keys. Apple has no page that says a Carbon
  hot key needs no permission: not confirmed in Apple's words.
- [S3] `CGRequestListenEventAccess`, macOS 10.15:
  https://developer.apple.com/documentation/coregraphics/cgrequestlisteneventaccess()
- [S4] Apple, "Control access to input monitoring on Mac":
  https://support.apple.com/guide/mac-help/control-access-to-input-monitoring-on-mac-mchl4cedafb6/mac
- [S5] `NSEvent.addGlobalMonitorForEvents`: "Key-related events may only be monitored if accessibility is enabled",
  and "you can only observe the event":
  https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:)
- [S6] Microsoft, `RegisterHotKey`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey
- [S7] `global-hotkey`, `src/platform_impl/windows/mod.rs`, branch `dev`: `MOD_NOREPEAT`, and a thread that calls
  `GetAsyncKeyState` every 50 ms: https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/windows/mod.rs
- [S8] Microsoft, `LowLevelKeyboardProc`: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc
- [S9] `global-hotkey`, `README.md` and `Cargo.toml`, branch `dev` (version 0.10.0, Rust 1.77):
  https://github.com/tauri-apps/global-hotkey
- [S10] Apple, "Mac keyboard shortcuts": https://support.apple.com/en-us/102650
- [S11] `SFSpeechRecognizer`: macOS 10.15; one language each; "stops speech recognition tasks that last longer than
  one minute": https://developer.apple.com/documentation/speech/sfspeechrecognizer
- [S12] `requiresOnDeviceRecognition`:
  https://developer.apple.com/documentation/speech/sfspeechrecognitionrequest/requiresondevicerecognition
- [S13] `objc2-speech` 0.3.2 and `objc2-avf-audio` 0.3.2, each for `objc2` from 0.6.2: https://docs.rs/objc2-speech
  and https://docs.rs/objc2-avf-audio . The lock file holds `objc2` 0.6.5 and `block2` 0.6.2 (`Cargo.lock`). That
  the two properties and the tap are in the bindings was not read method by method: not confirmed.
- [S14] `SpeechAnalyzer`, `final actor`, Swift only, 26.0: https://developer.apple.com/documentation/speech/speechanalyzer
- [S15] Apple, "Asking permission to use speech recognition":
  https://developer.apple.com/documentation/speech/asking-permission-to-use-speech-recognition . Also: "SpeechAnalyzer
  transcriber modules don't send audio data of the user's voice to Apple's servers".
- [S16] `SpeechTranscriber`: https://developer.apple.com/documentation/speech/speechtranscriber
- [S17] `whisper.cpp`, `README.md`: models 75 MiB (`tiny`), 142 MiB, 466 MiB, 1.5 GiB, 2.9 GiB; MIT; Metal on Apple
  chips: https://github.com/ggml-org/whisper.cpp
- [S18] `whisper-rs`: the Unlicense; its GitHub repository was archived on 2025-07-30 and it moved to
  https://codeberg.org/tazz4843/whisper-rs
- [S19] `cpal`, `README.md`: CoreAudio and WASAPI, Apache-2.0: https://github.com/RustAudio/cpal
- [S20] Microsoft, "Speech recognition": https://learn.microsoft.com/en-us/windows/apps/design/input/speech-recognition
- [S21] `NSMicrophoneUsageDescription`:
  https://developer.apple.com/documentation/bundleresources/information-property-list/nsmicrophoneusagedescription
- [S22] Apple, TN3127, "Inside Code Signing: Requirements":
  https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements
- [S23] Apple Developer Forums, thread 795739, an Apple engineer's answer of August 2025:
  https://developer.apple.com/forums/thread/795739
- [S24] GPUI at the pin: `crates/gpui/src/platform.rs` (`WindowOptions` from line 2602, `WindowKind` from 2839,
  `WindowBackgroundAppearance` from 2899); `crates/gpui_macos/src/window.rs` (lines 1043 to 1045, 1255 to 1264,
  1314 to 1318); `crates/gpui_windows/src/window.rs` (lines 494, 495 and 583); `crates/gpui/src/app.rs` (`QuitMode`
  from line 526, `on_reopen` at 369).
