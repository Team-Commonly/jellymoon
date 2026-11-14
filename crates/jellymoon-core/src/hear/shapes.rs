//! The wire shapes of a hearing: the events `jellymoon hear` reads and the lines it prints. An event with a name
//! or a kind this build does not know is no event. `tests/fixtures/hear-events.jsonl` and `hear-lines.jsonl` are
//! their examples.

use std::fmt;

use serde::{Deserialize, Serialize};

/// One event: the key, what the ear said, or the time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum HearEvent {
    /// `{"key":"down"}`, `{"key":"up"}`.
    Key {
        key: KeyMove,
    },
    /// `{"tick":1200}`: the time, in milliseconds of any clock that does not go back.
    Tick {
        tick: u64,
    },
    Said(Said),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyMove {
    Down,
    Up,
}

/// What the ear said: untrusted text, or why it cannot hear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "said", rename_all = "snake_case", deny_unknown_fields)]
pub enum Said {
    /// `{"said":"partial","words":"make the up"}`: the words so far, whole each time.
    Partial { words: String },
    /// `{"said":"final","words":"make the uploader retry"}`: the words of the hold that is over.
    Final { words: String },
    /// `{"said":"cannot","why":"microphone_refused"}`.
    Cannot { why: EarCannot },
}

/// Why the ear cannot hear. Each is said in core's own sentence ([`EarCannot::says`]), never in the ear's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarCannot {
    /// This system has no ear yet.
    NoEar,
    NoMicrophone,
    MicrophoneRefused,
    SpeechRefused,
    /// The system would send the sound away to recognize it.
    NotOnDevice,
    NoSuchLanguage,
    /// The app was not started from its bundle, so the system cannot ask for the microphone.
    NotBundled,
    Failed,
}

impl EarCannot {
    pub const ALL: [EarCannot; 8] = [
        EarCannot::NoEar,
        EarCannot::NoMicrophone,
        EarCannot::MicrophoneRefused,
        EarCannot::SpeechRefused,
        EarCannot::NotOnDevice,
        EarCannot::NoSuchLanguage,
        EarCannot::NotBundled,
        EarCannot::Failed,
    ];

    /// The sentence the jellyfish says.
    pub fn says(self) -> &'static str {
        match self {
            EarCannot::NoEar => "voice does not work on this system yet: type the wish on the page",
            EarCannot::NoMicrophone => "nothing was heard: no microphone was found",
            EarCannot::MicrophoneRefused => {
                "nothing was heard: Jellymoon may not use the microphone; allow it in System Settings, Privacy & \
                 Security, Microphone"
            }
            EarCannot::SpeechRefused => {
                "nothing was heard: Jellymoon may not recognize speech; allow it in System Settings, Privacy & \
                 Security, Speech Recognition"
            }
            EarCannot::NotOnDevice => {
                "nothing was heard: this Mac cannot recognize that language on the device, and Jellymoon sends no \
                 sound anywhere"
            }
            EarCannot::NoSuchLanguage => "nothing was heard: the language in voice.toml is not one this Mac recognizes",
            EarCannot::NotBundled => "nothing was heard: voice works only in Jellymoon.app, started as an app",
            EarCannot::Failed => "nothing was heard: the recognizer failed",
        }
    }
}

/// Which of its states the jellyfish is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JellyState {
    Quiet,
    Listening,
    Heard,
    Working,
    Asks,
    Waits,
    Cannot,
}

/// What the jellyfish shows: one line of `jellymoon hear`. The shell draws it and decides nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Jelly {
    pub state: JellyState,
    /// Core's own sentence for the state. Empty when the state says nothing.
    pub says: String,
    /// The words so far while listening, the heard wish while heard: through the door, at most 200 characters.
    pub words: String,
    /// The project a heard wish will be made for.
    pub project: String,
    /// The milliseconds a heard wish is still shown for. 0 in every other state.
    pub left: u64,
    /// How many of the day's runs are going. Left out when none is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub going: Option<usize>,
    /// The question a run asks, as the door left it. Only with the state `asks`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// The id of the wish that was just made: on the first line after it, and on no other.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made: Option<String>,
}

impl Jelly {
    /// The jellyfish at rest.
    pub fn quiet() -> Jelly {
        let (says, words, project) = (String::new(), String::new(), String::new());
        Jelly { state: JellyState::Quiet, says, words, project, left: 0, going: None, question: None, made: None }
    }
}

impl fmt::Display for Jelly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}
