//! Reads a page-data file. The shapes are core's and core's types refuse what does not fit them: a missing field or a
//! wrong type means there is no page. Two things are kept apart instead of refused, because a file written by a newer
//! core may hold them: a wish in a state this app does not know, and a top-level part it has never heard of. Each
//! becomes an [`Unknown`], which the scene draws as a plain note (ADR 0011, section 3).
//!
//! A page-data file is untrusted, so it is held to a size and each of its lists to a length before anything else looks
//! at it. A file over the size is refused. A list over its length is cut to it, and [`LeftOut`] counts what went.

use jellymoon_core::{PageData, Wish, WishState};
use serde_json::{Map, Value};

/// The largest page-data file read, in bytes: 1 MiB. Core's own page data stays under 10 KiB.
pub const MAX_PAGE_BYTES: usize = 1024 * 1024;

/// The most wishes, clashes, stars of one wish, and items of unknown parts (all parts together) that are drawn. Core
/// writes at most 8, 2 and 4, and no unknown part.
pub const MAX_WISHES: usize = 24;
pub const MAX_CLASHES: usize = 8;
pub const MAX_STARS: usize = 12;
pub const MAX_UNKNOWN: usize = 8;

/// What the app draws: core's page data, what the file held that this app has no piece for, and what it held beyond
/// the lengths above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub data: PageData,
    pub unknown: Vec<Unknown>,
    pub left_out: LeftOut,
}

/// How many of each thing the file held beyond what is drawn. `stars` counts those of the wishes that are kept.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LeftOut {
    pub wishes: usize,
    pub clashes: usize,
    pub stars: usize,
    pub unknown: usize,
}

/// Something in a page-data file that this app has no piece for: what to call it, and its text when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    pub title: String,
    pub body: String,
}

/// The parts of page data this app knows.
const KNOWN: [&str; 4] = ["day", "wishes", "clashes", "read"];

/// [`page`] for a file's bytes, which are refused when they are over the size or are not text.
pub fn bytes(bytes: Vec<u8>) -> Result<Page, String> {
    if bytes.len() > MAX_PAGE_BYTES {
        return Err(OVER.to_string());
    }
    page(&String::from_utf8(bytes).map_err(|_| "it is not UTF-8 text".to_string())?)
}

const OVER: &str = "it is over 1 MiB";

/// The page a file's text holds, or one sentence on why it is not page data.
pub fn page(text: &str) -> Result<Page, String> {
    if text.len() > MAX_PAGE_BYTES {
        return Err(OVER.to_string());
    }
    let mut file: Map<String, Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut left_out = LeftOut {
        wishes: cut(file.get_mut("wishes"), MAX_WISHES),
        clashes: cut(file.get_mut("clashes"), MAX_CLASHES),
        ..LeftOut::default()
    };
    let mut unknown = Vec::new();
    if let Some(Value::Array(wishes)) = file.get_mut("wishes") {
        left_out.stars = wishes.iter_mut().map(|wish| cut(wish.get_mut("stars"), MAX_STARS)).sum();
        wishes.retain(|wish| match unknown_state(wish) {
            Some(found) => {
                unknown.push(found);
                false
            }
            None => true,
        });
    }
    let mut parts: Vec<(&String, &Value)> = file.iter().filter(|(part, _)| !KNOWN.contains(&part.as_str())).collect();
    parts.sort_by_key(|(part, _)| *part);
    let items = |value: &'_ Value| value.as_array().map_or(1, Vec::len);
    left_out.unknown = parts.iter().map(|(_, value)| items(value)).sum::<usize>().saturating_sub(MAX_UNKNOWN);
    let each = parts.iter().flat_map(|(part, value)| {
        let items = value.as_array().map_or(std::slice::from_ref(*value), Vec::as_slice);
        items.iter().map(move |item| Unknown { title: (*part).clone(), body: text_of(item) })
    });
    unknown.extend(each.take(MAX_UNKNOWN));
    let data = serde_json::from_value(Value::Object(file)).map_err(|e| e.to_string())?;
    Ok(Page { data, unknown, left_out })
}

/// Cuts a list to its first `most` items and says how many went. What is not a list is left for core's types to refuse.
fn cut(list: Option<&mut Value>, most: usize) -> usize {
    let Some(Value::Array(items)) = list else {
        return 0;
    };
    let over = items.len().saturating_sub(most);
    items.truncate(most);
    over
}

/// A wish that is whole except that its state is a word this app does not know.
fn unknown_state(wish: &Value) -> Option<Unknown> {
    let state = wish.get("state")?.as_str()?;
    if serde_json::from_value::<WishState>(Value::String(state.to_string())).is_ok() {
        return None;
    }
    let mut known = wish.clone();
    *known.get_mut("state")? = Value::String("working".to_string());
    let wish: Wish = serde_json::from_value(known).ok()?;
    Some(Unknown { title: wish.label, body: format!("A wish in a state this app cannot draw yet: {state}") })
}

/// The words of an item of an unknown part: the item itself when it is text, else its `label` or `text`.
fn text_of(item: &Value) -> String {
    let named = |key: &str| item.get(key).and_then(Value::as_str);
    item.as_str().or_else(|| named("label")).or_else(|| named("text")).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn file() -> Value {
        json!({
            "day": "2026-03-10",
            "wishes": [
                { "id": "w1", "label": "Login page", "said": "fix the login page", "project": "beta", "state": "working", "stars": [] },
            ],
            "clashes": [],
            "read": { "turns": 2, "projects": ["beta"] },
        })
    }

    fn read(file: &Value) -> Result<Page, String> {
        page(&file.to_string())
    }

    #[test]
    fn a_file_of_page_data_reads_as_core_reads_it() {
        let read = read(&file()).unwrap();
        assert_eq!(read.data, serde_json::from_value::<PageData>(file()).unwrap());
        assert_eq!(read.unknown, []);
    }

    #[test]
    fn a_wish_in_an_unknown_state_is_set_aside_and_the_rest_is_kept() {
        let mut file = file();
        let mut paused = file["wishes"][0].clone();
        paused["id"] = json!("w2");
        paused["label"] = json!("Faster search");
        paused["state"] = json!("paused");
        file["wishes"].as_array_mut().unwrap().push(paused);
        let read = read(&file).unwrap();
        assert_eq!(read.data.wishes.iter().map(|wish| wish.id.as_str()).collect::<Vec<_>>(), ["w1"]);
        let body = "A wish in a state this app cannot draw yet: paused".to_string();
        assert_eq!(read.unknown, [Unknown { title: "Faster search".to_string(), body }]);
    }

    #[test]
    fn a_wish_that_is_broken_in_any_other_way_refuses_the_file() {
        for (field, value) in [("state", json!(7)), ("stars", json!("one")), ("label", Value::Null)] {
            let mut file = file();
            file["wishes"][0][field] = value;
            assert!(read(&file).is_err(), "{field}");
        }
        let mut both = file();
        both["wishes"][0]["state"] = json!("paused");
        both["wishes"][0].as_object_mut().unwrap().remove("said");
        assert!(read(&both).is_err(), "an unknown state does not excuse a missing field");
    }

    #[test]
    fn a_part_the_app_has_never_heard_of_is_set_aside_item_by_item() {
        let mut file = file();
        file["reminders"] = json!([{ "label": "Water the plants" }, { "text": "Call the plumber" }, "Stretch", 4]);
        file["mood"] = json!("sunny");
        let unknown = read(&file).unwrap().unknown;
        let pairs: Vec<(&str, &str)> = unknown.iter().map(|one| (one.title.as_str(), one.body.as_str())).collect();
        assert_eq!(
            pairs,
            [
                ("mood", "sunny"),
                ("reminders", "Water the plants"),
                ("reminders", "Call the plumber"),
                ("reminders", "Stretch"),
                ("reminders", "")
            ]
        );
    }

    #[test]
    fn a_list_over_its_length_is_cut_to_it_and_what_went_is_counted() {
        let mut page_file = file();
        let wish = |i: usize, state: &str, stars: usize| {
            let mut wish = file()["wishes"][0].clone();
            (wish["id"], wish["state"], wish["stars"]) =
                (json!(format!("w{i}")), json!(state), json!(vec!["a"; stars]));
            wish
        };
        // The 25th wish and the 9th clash are broken, and are never looked at.
        let mut wishes: Vec<Value> = (0..24).map(|i| wish(i, if i == 3 { "paused" } else { "done" }, i)).collect();
        wishes.extend([json!({ "id": 7 }), wish(26, "paused", 40)]);
        page_file["wishes"] = json!(wishes);
        let clash = json!({ "a": "w1", "b": "w2", "quote_a": "x", "quote_b": "y", "settled": false });
        page_file["clashes"] = json!([clash, clash, clash, clash, clash, clash, clash, clash, 9, clash, clash]);
        page_file["b"] = json!(["1", "2", "3", "4", "5", "6", "7"]);
        page_file["a"] = json!("0");
        page_file["c"] = json!(["8", "9"]);
        let read = read(&page_file).unwrap();
        assert_eq!((read.data.wishes.len(), read.data.clashes.len()), (23, MAX_CLASHES), "one of 24 is paused");
        assert!(read.data.wishes.iter().all(|wish| wish.stars.len() <= MAX_STARS));
        assert_eq!(read.left_out, LeftOut { wishes: 2, clashes: 3, stars: (13..24).map(|i| i - 12).sum(), unknown: 2 });
        let kept: Vec<&str> = read.unknown.iter().skip(1).map(|one| one.body.as_str()).collect();
        assert_eq!(kept, ["0", "1", "2", "3", "4", "5", "6", "7"], "the first items of the parts in name order");

        assert_eq!(page(&file().to_string()).unwrap().left_out, LeftOut::default());
        assert_eq!(bytes(vec![b' '; MAX_PAGE_BYTES + 1]).unwrap_err(), "it is over 1 MiB");
        assert_eq!(page(&" ".repeat(MAX_PAGE_BYTES + 1)).unwrap_err(), "it is over 1 MiB");
        assert_eq!(bytes(vec![0xff, 0xfe]).unwrap_err(), "it is not UTF-8 text");
    }

    #[test]
    fn text_that_is_not_page_data_is_refused_with_a_reason() {
        for text in
            ["", "[]", "{}", r#"{"day":"2026-03-10","wishes":{},"clashes":[],"read":{"turns":0,"projects":[]}}"#]
        {
            assert!(!page(text).unwrap_err().is_empty(), "{text}");
        }
    }
}
