//! Markdown Preview domain state. No OS, network, JavaScript or persistent storage.
pub mod html;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};

pub const COMMAND: &str = "preview-markdown";
pub const SLUG: &str = "markdown-preview";
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
pub const MAX_HTML_BYTES: usize = 8 * 1024 * 1024;
static SERIAL: AtomicU64 = AtomicU64::new(1);
static PROCESS: OnceLock<String> = OnceLock::new();
fn token() -> String {
    let process = PROCESS.get_or_init(|| uuid::Uuid::new_v4().to_string());
    format!("md:{process}:{}", SERIAL.fetch_add(1, Ordering::Relaxed))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Page {
    Editor,
    Preview,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Change,
    Preview,
    Clear,
    Back,
    CopyMarkdown,
    CopyHtml,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CopyState {
    #[default]
    None,
    // Checkpoint before await: a dropped future has an uncertain outcome.
    Pending(Action),
    Uncertain(Action),
    Acknowledged(Action),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub page: Page,
    pub draft: String,
    pub preview: String,
    pub html: String,
    pub copy: CopyState,
    pub capabilities: BTreeMap<String, Action>,
    pub disposed: bool,
    // Omit controlled value after a callback so newer local keystrokes survive.
    pub emit_value: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    View,
    Unchanged,
    Copy(String),
}
impl Default for Session {
    fn default() -> Self {
        let mut state = Self {
            page: Page::Editor,
            draft: String::new(),
            preview: String::new(),
            html: String::new(),
            copy: CopyState::None,
            capabilities: BTreeMap::new(),
            disposed: false,
            emit_value: true,
        };
        state.rekey();
        state
    }
}
impl Session {
    fn rekey(&mut self) {
        self.capabilities.clear();
        let actions: &[Action] = match self.page {
            Page::Editor => &[Action::Change, Action::Preview, Action::Clear],
            Page::Preview => &[Action::Back, Action::CopyMarkdown, Action::CopyHtml],
        };
        for action in actions {
            self.capabilities.insert(token(), *action);
        }
    }
    pub fn id(&self, action: Action) -> &str {
        self.capabilities
            .iter()
            .find(|(_, a)| **a == action)
            .map(|(id, _)| id.as_str())
            .unwrap_or("")
    }
    pub fn invoke(&mut self, payload: &Value) -> Result<Outcome, String> {
        if self.disposed {
            return Err("Markdown Preview session has ended".into());
        }
        let id = payload
            .get("actionId")
            .and_then(Value::as_str)
            .ok_or("Missing action capability")?;
        let action = *self
            .capabilities
            .get(id)
            .ok_or("Stale or invalid Markdown Preview action; reopen the action panel")?;
        // Fully validate before consuming any capability or altering a legitimate draft.
        let value = match action {
            Action::Change => Some(text_field(payload, "value")?),
            Action::Preview => Some(text_field(payload, "markdown")?),
            _ => None,
        };
        match action {
            Action::Change => {
                self.draft = value.unwrap().into();
                self.emit_value = false;
                Ok(Outcome::View)
            }
            Action::Preview => {
                let text = value.unwrap();
                self.draft = text.into();
                self.emit_value = false;
                if text.trim_matches(js_whitespace).is_empty() {
                    return Ok(Outcome::View);
                }
                // A conversion/resource failure keeps the exact submitted draft editable.
                let html = html::render(text)?;
                self.preview = text.into();
                self.html = html;
                self.page = Page::Preview;
                self.copy = CopyState::None;
                self.rekey();
                Ok(Outcome::View)
            }
            Action::Clear => {
                self.draft.clear();
                self.preview.clear();
                self.html.clear();
                self.copy = CopyState::None;
                self.emit_value = true;
                self.rekey();
                Ok(Outcome::View)
            }
            Action::Back => {
                self.page = Page::Editor;
                self.emit_value = true;
                self.copy = CopyState::None;
                self.rekey();
                Ok(Outcome::View)
            }
            Action::CopyMarkdown | Action::CopyHtml => {
                let target = if action == Action::CopyMarkdown {
                    self.preview.clone()
                } else {
                    self.html.clone()
                };
                self.copy = CopyState::Pending(action);
                self.rekey();
                Ok(Outcome::Copy(target))
            }
        }
    }
    pub fn finish_copy(&mut self, acknowledged: bool) {
        if let CopyState::Pending(action) = self.copy {
            self.copy = if acknowledged {
                CopyState::Acknowledged(action)
            } else {
                CopyState::Uncertain(action)
            };
        }
    }
    pub fn dispose(&mut self) {
        self.disposed = true;
        self.capabilities.clear();
        self.draft.clear();
        self.preview.clear();
        self.html.clear();
        self.copy = CopyState::None;
    }
    pub fn view(&self) -> Value {
        if self.disposed {
            return json!({"ok":true,"mode":"no-view","message":"Markdown Preview closed"});
        }
        let action = |kind: Action, title: &str| json!({"id":self.id(kind),"title":title,"kind":if kind==Action::Preview {"submit-form"} else {"action"}});
        match self.page {
            Page::Editor => {
                let mut field = json!({"type":"Form.TextArea","props":{"id":"markdown","title":"Markdown Content","info":"Examples: # Heading 1, **Bold text**, *italic text*, - Bullet list, [Link](url), `inline code`","placeholder":"Enter your Markdown content here...","actionId":self.id(Action::Change)}});
                if self.emit_value {
                    field["props"]["value"] = json!(self.draft);
                }
                json!({"ok":true,"mode":"view","title":"Markdown Editor","root":{"type":"Form","props":{"navigationTitle":"Markdown Editor"},"children":[field]},"actions":[action(Action::Preview,"Preview Markdown"),action(Action::Clear,"Clear")]})
            }
            Page::Preview => {
                let stats = statistics(&self.preview);
                let mut labels = vec![
                    label("Characters", stats.characters.to_string()),
                    label("Words", stats.words.to_string()),
                    label("Lines", stats.lines.to_string()),
                ];
                let message = match self.copy {
                    CopyState::None => None,
                    CopyState::Pending(_) | CopyState::Uncertain(_) => {
                        Some("Copy outcome uncertain. Use a copy action to retry explicitly.")
                    }
                    CopyState::Acknowledged(_) => Some("Copied to clipboard"),
                };
                if let Some(message) = message {
                    labels.push(label("Copy status", message.into()));
                }
                let uncertain =
                    matches!(self.copy, CopyState::Pending(_) | CopyState::Uncertain(_));
                json!({"ok":true,"mode":"view","title":"Markdown Preview","root":{"type":"Detail","props":{"navigationTitle":"Markdown Preview","markdown":self.preview,"backActionId":self.id(Action::Back)},"metadata":{"type":"Detail.Metadata","children":labels}},"actions":[action(Action::Back,"Back to Editor"),action(Action::CopyMarkdown,if uncertain {"Retry Copy Markdown"} else {"Copy Markdown"}),action(Action::CopyHtml,if uncertain {"Retry Copy HTML"} else {"Copy HTML"})]})
            }
        }
    }
}
fn label(title: &str, text: String) -> Value {
    json!({"type":"Detail.Metadata.Label","props":{"title":title,"text":text}})
}
fn text_field<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    let text = payload
        .get("formValues")
        .and_then(Value::as_object)
        .and_then(|m| m.get(key))
        .and_then(Value::as_str)
        .ok_or("Markdown field must be a string in formValues")?;
    if text.len() > MAX_INPUT_BYTES {
        return Err("Markdown exceeds the 1 MiB input limit; nothing was truncated".into());
    }
    Ok(text)
}
// ECMAScript WhiteSpace + LineTerminator, deliberately excludes Rust's U+0085.
pub fn js_whitespace(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Statistics {
    pub characters: usize,
    pub words: usize,
    pub lines: usize,
}
pub fn statistics(text: &str) -> Statistics {
    Statistics {
        characters: text.encode_utf16().count(),
        words: text.split(js_whitespace).filter(|w| !w.is_empty()).count(),
        lines: text.split('\n').count(),
    }
}
/// Host effects require positive, non-contradictory object acknowledgement.
pub fn acknowledged(result: &Result<Value, String>) -> bool {
    result
        .as_ref()
        .ok()
        .and_then(Value::as_object)
        .is_some_and(|m| {
            m.get("ok") == Some(&Value::Bool(true))
                && m.get("error").is_none_or(Value::is_null)
                && m.get("success").is_none_or(|v| v == &Value::Bool(true))
        })
}
