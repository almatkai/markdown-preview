use serde_json::{json, Value};
use tezbar_native_markdown_preview::{
    acknowledged, html, Action, CopyState, Outcome, Page, Session, MAX_INPUT_BYTES,
};
fn invoke(s: &mut Session, a: Action, values: Value) -> Result<Outcome, String> {
    s.invoke(&json!({"actionId":s.id(a),"formValues":values}))
}
#[test]
fn acknowledgements_are_strict_and_never_use_private_host_error_text() {
    for value in [
        json!(null),
        json!(true),
        json!(false),
        json!({}),
        json!({"ok":false}),
        json!({"ok":true,"error":"private"}),
        json!({"ok":true,"error":false}),
        json!({"ok":true,"success":false}),
        json!({"ok":true,"success":null}),
        json!({"ok":1}),
        json!([]),
    ] {
        assert!(!acknowledged(&Ok(value)));
    }
    for value in [
        json!({"ok":true}),
        json!({"ok":true,"error":null}),
        json!({"ok":true,"success":true}),
        json!({"ok":true,"error":null,"success":true}),
    ] {
        assert!(acknowledged(&Ok(value)));
    }
    assert!(!acknowledged(&Err("private token".into())));
}
#[test]
fn malformed_events_do_not_consume_capabilities_or_drafts() {
    let mut s = Session::default();
    let before = s.clone();
    for p in [
        json!(null),
        json!({}),
        json!({"actionId":12}),
        json!({"actionId":"__nav_pop__"}),
        json!({"actionId":s.id(Action::Change),"formValues":{"markdown":"wrong"}}),
        json!({"actionId":s.id(Action::Preview),"formValues":{"markdown":2}}),
    ] {
        assert!(s.invoke(&p).is_err());
        assert_eq!(s, before);
    }
    invoke(&mut s, Action::Preview, json!({"markdown":"correct"})).unwrap();
    assert_eq!(s.preview, "correct");
}
#[test]
fn callbacks_keep_stable_field_identity_and_omit_controlled_value() {
    let mut s = Session::default();
    let id = s.id(Action::Change).to_string();
    invoke(&mut s, Action::Change, json!({"value":"one"})).unwrap();
    assert_eq!(s.id(Action::Change), id);
    assert_eq!(s.view()["root"]["children"][0]["props"]["id"], "markdown");
    assert!(s.view()["root"]["children"][0]["props"]
        .get("value")
        .is_none());
    invoke(&mut s, Action::Change, json!({"value":"newer"})).unwrap();
    assert_eq!(s.draft, "newer");
}
#[test]
fn complete_submit_map_wins_over_older_field_callback() {
    let mut s = Session::default();
    invoke(&mut s, Action::Change, json!({"value":"old"})).unwrap();
    let stale = s.id(Action::Change).to_string();
    invoke(
        &mut s,
        Action::Preview,
        json!({"markdown":"  # latest\n😀  "}),
    )
    .unwrap();
    assert_eq!(s.preview, "  # latest\n😀  ");
    assert!(s
        .invoke(&json!({"actionId":stale,"formValues":{"value":"old callback"}}))
        .is_err());
    assert_eq!(s.preview, "  # latest\n😀  ");
}
#[test]
fn back_and_clear_preserve_and_reset_exact_source_semantics() {
    let mut s = Session::default();
    let text = "\r\n\t<&> 日本 😀  ";
    invoke(&mut s, Action::Preview, json!({"markdown":text})).unwrap();
    invoke(&mut s, Action::Back, json!({})).unwrap();
    assert_eq!(s.draft, text);
    assert_eq!(s.view()["root"]["children"][0]["props"]["value"], text);
    let stale = s.id(Action::Change).to_string();
    invoke(&mut s, Action::Clear, json!({})).unwrap();
    assert_eq!(s.draft, "");
    assert_eq!(s.view()["root"]["children"][0]["props"]["value"], "");
    assert!(s
        .invoke(&json!({"actionId":stale,"formValues":{"value":"revive"}}))
        .is_err());
}
#[test]
fn whitespace_submit_stays_in_editor_and_keeps_exact_draft() {
    let mut s = Session::default();
    let id = s.id(Action::Preview).to_string();
    invoke(&mut s, Action::Preview, json!({"markdown":" \t\u{feff}\n"})).unwrap();
    assert_eq!(s.page, Page::Editor);
    assert_eq!(s.draft, " \t\u{feff}\n");
    assert_eq!(s.id(Action::Preview), id);
}
#[test]
fn oversized_input_fails_explicitly_without_truncating_existing_draft() {
    let mut s = Session::default();
    invoke(&mut s, Action::Change, json!({"value":"keep"})).unwrap();
    let before = s.clone();
    let error = invoke(
        &mut s,
        Action::Preview,
        json!({"markdown":"😀".repeat(MAX_INPUT_BYTES/4+1)}),
    )
    .unwrap_err();
    assert!(error.contains("nothing was truncated"));
    assert_eq!(s, before);
}
#[test]
fn excessive_depth_keeps_submitted_draft_for_recovery() {
    let mut s = Session::default();
    let text = format!("{}x", "> ".repeat(130));
    let error = invoke(&mut s, Action::Preview, json!({"markdown":text})).unwrap_err();
    assert!(error.contains("nesting"));
    assert_eq!(s.draft, text);
    assert_eq!(s.page, Page::Editor);
}
#[test]
fn html_output_limit_is_explicit() {
    let input = "- [x]\n".repeat(170000);
    assert!(input.len() <= MAX_INPUT_BYTES);
    assert!(matches!(html::render(&input),Err(e) if e.contains("8 MiB")));
}
#[test]
fn copy_is_checkpointed_and_bound_to_committed_text() {
    let mut s = Session::default();
    invoke(&mut s, Action::Preview, json!({"markdown":"# original"})).unwrap();
    let old = s.id(Action::CopyHtml).to_string();
    let expected = s.html.clone();
    let result=s.invoke(&json!({"actionId":old,"formValues":{"content":"forged","url":"https://attacker.invalid"},"output":"forged"})).unwrap();
    assert_eq!(result, Outcome::Copy(expected));
    assert_eq!(s.copy, CopyState::Pending(Action::CopyHtml));
    assert!(s.invoke(&json!({"actionId":old})).is_err());
    let restored: Session = serde_json::from_value(json!(s)).unwrap();
    assert!(restored.view().to_string().contains("outcome uncertain"));
    assert_ne!(restored.id(Action::CopyHtml), old);
}
#[test]
fn failed_copy_requires_fresh_explicit_retry_then_acknowledges() {
    let mut s = Session::default();
    invoke(&mut s, Action::Preview, json!({"markdown":"text"})).unwrap();
    invoke(&mut s, Action::CopyMarkdown, json!({})).unwrap();
    s.finish_copy(false);
    assert_eq!(s.copy, CopyState::Uncertain(Action::CopyMarkdown));
    assert!(s.view().to_string().contains("Retry Copy"));
    invoke(&mut s, Action::CopyMarkdown, json!({})).unwrap();
    s.finish_copy(true);
    assert_eq!(s.copy, CopyState::Acknowledged(Action::CopyMarkdown));
    let before = s.clone();
    assert!(s.invoke(&json!({"actionId":"forged"})).is_err());
    assert_eq!(s, before);
}
#[test]
fn cross_session_and_discarded_clone_actions_cannot_revive() {
    let mut s = Session::default();
    let other = Session::default();
    assert!(s
        .invoke(&json!({"actionId":other.id(Action::Preview),"formValues":{"markdown":"cross"}}))
        .is_err());
    let mut staged = s.clone();
    invoke(
        &mut staged,
        Action::Preview,
        json!({"markdown":"discarded"}),
    )
    .unwrap();
    let discarded = staged.id(Action::Back).to_string();
    invoke(&mut s, Action::Preview, json!({"markdown":"committed"})).unwrap();
    assert_ne!(s.id(Action::Back), discarded);
    assert!(s.invoke(&json!({"actionId":discarded})).is_err());
}
#[test]
fn capabilities_are_unique_across_concurrent_sessions() {
    let threads: Vec<_> = (0..24)
        .map(|_| {
            std::thread::spawn(|| {
                Session::default()
                    .capabilities
                    .into_keys()
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let ids: Vec<_> = threads
        .into_iter()
        .flat_map(|t| t.join().unwrap())
        .collect();
    let distinct: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(distinct.len(), ids.len());
}
#[test]
fn dispose_clears_private_content_and_prevents_reuse() {
    let mut s = Session::default();
    invoke(&mut s, Action::Preview, json!({"markdown":"private"})).unwrap();
    let old = s.id(Action::Back).to_string();
    s.dispose();
    assert_eq!(s.draft, "");
    assert_eq!(s.preview, "");
    assert_eq!(s.html, "");
    assert!(s.capabilities.is_empty());
    assert!(s.invoke(&json!({"actionId":old})).is_err());
}
