//! Reads a page-data file. The shapes are core's and core's types refuse what does not fit them: a missing field or a
//! wrong type means there is no page. Two things are kept apart instead of refused, because a file written by a newer
//! core may hold them: a wish in a state this app does not know, and a top-level part it has never heard of. Each
//! becomes an [`Unknown`], which the scene draws as a plain note (ADR 0011, section 3).

use jellymoon_core::{PageData, Wish, WishState};
use serde_json::{Map, Value};

/// What the app draws: core's page data, and what the file held that this app has no piece for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub data: PageData,
    pub unknown: Vec<Unknown>,
}

/// Something in a page-data file that this app has no piece for: what to call it, and its text when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    pub title: String,
    pub body: String,
}

/// The parts of page data this app knows.
const KNOWN: [&str; 4] = ["day", "wishes", "clashes", "read"];

/// The page a file's text holds, or one sentence on why it is not page data.
pub fn page(text: &str) -> Result<Page, String> {
    let mut file: Map<String, Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut unknown = Vec::new();
    if let Some(Value::Array(wishes)) = file.get_mut("wishes") {
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
    for (part, value) in parts {
        let items = value.as_array().map_or(std::slice::from_ref(value), Vec::as_slice);
        unknown.extend(items.iter().map(|item| Unknown { title: part.clone(), body: text_of(item) }));
    }
    let data = serde_json::from_value(Value::Object(file)).map_err(|e| e.to_string())?;
    Ok(Page { data, unknown })
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
    fn text_that_is_not_page_data_is_refused_with_a_reason() {
        for text in
            ["", "[]", "{}", r#"{"day":"2026-03-10","wishes":{},"clashes":[],"read":{"turns":0,"projects":[]}}"#]
        {
            assert!(!page(text).unwrap_err().is_empty(), "{text}");
        }
    }
}
