use serde_json::json;
use tezbar_native_markdown_preview::{Action, Session};
fn main() {
    let mut s = Session::default();
    let editor = s.view();
    s.invoke(&json!({"actionId":s.id(Action::Preview),"formValues":{"markdown":"# Heading 1"}}))
        .unwrap();
    println!("{}", json!({"editor":editor,"preview":s.view()}));
}
