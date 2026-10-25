use serde_json::{json, Value};
use tezbar_native_markdown_preview::{html, statistics, Action, Outcome, Session};
fn oracle() -> Value {
    serde_json::from_str(include_str!("../fixtures/source-oracle.json")).unwrap()
}
#[test]
fn source_statistics_text_navigation_and_defaults() {
    let oracle = oracle();
    let initial = Session::default();
    assert_eq!(initial.view()["title"], oracle["initial"]["title"]);
    for case in oracle["cases"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let stats = statistics(input);
        let mut session = Session::default();
        let submit =
            json!({"actionId":session.id(Action::Preview),"formValues":{"markdown":input}});
        session.invoke(&submit).unwrap();
        assert_eq!(
            session.page == tezbar_native_markdown_preview::Page::Preview,
            case["accepted"] == true,
            "case {}",
            case["index"]
        );
        if case["accepted"] == true {
            assert_eq!(session.preview, input);
            assert_eq!(
                stats.characters.to_string(),
                case["statistics"][0]["text"].as_str().unwrap()
            );
            assert_eq!(
                stats.words.to_string(),
                case["statistics"][1]["text"].as_str().unwrap()
            );
            assert_eq!(
                stats.lines.to_string(),
                case["statistics"][2]["text"].as_str().unwrap()
            );
            assert_eq!(session.invoke(&json!({"actionId":session.id(Action::CopyMarkdown),"formValues":{"content":"FORGED"}})).unwrap(),Outcome::Copy(input.into()));
            session.finish_copy(true);
            session
                .invoke(&json!({"actionId":session.id(Action::Back)}))
                .unwrap();
            assert_eq!(session.draft, input);
            session
                .invoke(&json!({"actionId":session.id(Action::Clear)}))
                .unwrap();
            assert_eq!(session.draft, "");
        }
    }
}
#[test]
fn source_html_parity() {
    let oracle = oracle();
    let mut mismatches = Vec::new();
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["accepted"] == true)
    {
        let input = case["input"].as_str().unwrap();
        let got = html::render(input).unwrap();
        if got != case["html"].as_str().unwrap() {
            mismatches.push(format!(
                "{} INPUT {:?}\nGOT {:?}\nWANT {:?}",
                case["index"], input, got, case["html"]
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
