use markdown_preview_beta_contract::native_core::{
    common::{CoreContext, CorePaths, CoreResult},
    extensions::{v2_markdown_preview as adapter, v2_sdk as sdk, Sessions},
};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tezbar_native_markdown_preview::{Action, CopyState, Session};
fn context(dir: &std::path::Path) -> CoreContext {
    CoreContext::new(
        CorePaths {
            data_dir: dir.join("data"),
            config_dir: dir.join("config"),
            resource_dir: dir.join("resources"),
            temp_dir: dir.join("tmp"),
            home_dir: dir.into(),
        },
        None,
    )
}
fn action(view: &Value, title: &str) -> String {
    view["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == title)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
async fn call(ctx: &CoreContext, op: &str, payload: Value, state: &mut Value) -> CoreResult {
    adapter::handle(ctx, "preview-markdown", op, &payload, state).await
}
async fn preview(ctx: &CoreContext, state: &mut Value, text: &str) -> Value {
    let view = call(
        ctx,
        "extension:run-command",
        json!({"argumentValues":{"ignored":"x"},"preferences":{"ignored":true}}),
        state,
    )
    .await
    .unwrap();
    call(
        ctx,
        "extension:invoke-action",
        json!({"actionId":action(&view,"Preview Markdown"),"formValues":{"markdown":text}}),
        state,
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn real_adapter_accepts_actual_command_payload_and_starts_empty_without_os_or_store() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    ctx.set_router(Arc::new(|c, _| panic!("Unexpected host effect: {c}")))
        .unwrap();
    let mut state = json!({});
    let v = call(
        &ctx,
        "extension:run-command",
        json!({"argumentValues":{"markdown":"ignored"},"preferences":{}}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(v["root"]["children"][0]["props"]["value"], "");
    assert!(!ctx.paths.data_dir.exists());
    assert_eq!(adapter::commands(), ["preview-markdown"]);
    assert!(
        adapter::handle(&ctx, "bad", "extension:run-command", &json!({}), &mut state)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn callback_contract_and_back_preserve_exact_draft() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let mut state = json!({});
    let v = call(&ctx, "extension:run-command", json!({}), &mut state)
        .await
        .unwrap();
    let field = v["root"]["children"][0]["props"]["actionId"]
        .as_str()
        .unwrap();
    let v = call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":field,"formValues":{"value":"first"}}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(v["root"]["children"][0]["props"].get("value").is_none());
    let v = call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Preview Markdown"),"formValues":{"markdown":"newest\r\n😀"}}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(v["root"]["props"]["markdown"], "newest\r\n😀");
    assert_eq!(
        v["root"]["props"]["backActionId"],
        action(&v, "Back to Editor")
    );
    let v = call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Back to Editor")}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(v["root"]["children"][0]["props"]["value"], "newest\r\n😀");
}
#[tokio::test]
async fn refresh_search_and_load_more_never_mint_uncommitted_ids() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let mut state = json!({});
    preview(&ctx, &mut state, "# text").await;
    let snapshot = state.clone();
    for op in [
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
    ] {
        let mut discarded = state.clone();
        call(&ctx, op, json!({"searchText":"anything"}), &mut discarded)
            .await
            .unwrap();
        assert_eq!(discarded, snapshot);
        assert_eq!(state, snapshot);
    }
    let a = call(&ctx, "extension:refresh-session", json!({}), &mut state)
        .await
        .unwrap();
    let b = call(&ctx, "extension:refresh-session", json!({}), &mut state)
        .await
        .unwrap();
    assert_eq!(a, b);
}
#[tokio::test]
async fn copy_only_uses_sdk_actual_clipboard_route_and_committed_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    ctx.set_router(Arc::new(move |c, a| {
        recorded.lock().unwrap().push((c, a));
        Box::pin(async { Ok(json!({"ok":true,"error":null,"success":true})) })
    }))
    .unwrap();
    let mut state = json!({});
    let v = preview(&ctx, &mut state, "# own\r\n日本 😀").await;
    let v = call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Copy Markdown"),"formValues":{"content":"forged"}}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(v.to_string().contains("Copied to clipboard"));
    let v = call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Copy HTML"),"url":"https://forged.invalid"}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(v.to_string().contains("Copied to clipboard"));
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            ("clipboard:write".into(), vec![json!("# own\r\n日本 😀")]),
            (
                "clipboard:write".into(),
                vec![json!("<h1>own</h1>\n<p>日本 😀</p>\n")]
            )
        ]
    );
}
#[tokio::test]
async fn every_malformed_or_failed_ack_is_uncertain_private_and_requires_explicit_retry() {
    let variants: Vec<CoreResult> = vec![
        Err("PRIVATE /Users/almat/secret credential".into()),
        Ok(json!(null)),
        Ok(json!(true)),
        Ok(json!({})),
        Ok(json!({"ok":true,"success":false})),
        Ok(json!({"ok":true,"error":"PRIVATE"})),
        Ok(json!({"ok":true,"error":false})),
        Ok(json!({"ok":true,"success":null})),
        Ok(json!({"ok":false})),
    ];
    for reply in variants {
        let dir = tempfile::tempdir().unwrap();
        let ctx = context(dir.path());
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        ctx.set_router(Arc::new(move |_, _| {
            c.fetch_add(1, Ordering::SeqCst);
            let reply = reply.clone();
            Box::pin(async move { reply })
        }))
        .unwrap();
        let mut state = json!({});
        let v = preview(&ctx, &mut state, "text").await;
        let old = action(&v, "Copy Markdown");
        let v = call(
            &ctx,
            "extension:invoke-action",
            json!({"actionId":old}),
            &mut state,
        )
        .await
        .unwrap();
        assert!(v.to_string().contains("outcome uncertain"));
        assert!(!v.to_string().contains("PRIVATE"));
        assert!(!v.to_string().contains("Copied to clipboard"));
        assert!(call(
            &ctx,
            "extension:invoke-action",
            json!({"actionId":old}),
            &mut state
        )
        .await
        .is_err());
        call(&ctx, "extension:refresh-session", json!({}), &mut state)
            .await
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        call(
            &ctx,
            "extension:invoke-action",
            json!({"actionId":action(&v,"Retry Copy Markdown")}),
            &mut state,
        )
        .await
        .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
#[tokio::test]
async fn interrupted_copy_has_checkpoint_and_explicit_recovery_capability() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    ctx.set_router(Arc::new(move |_, _| {
        c.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { std::future::pending().await })
    }))
    .unwrap();
    let mut state = json!({});
    let v = preview(&ctx, &mut state, "private draft").await;
    let old = action(&v, "Copy HTML");
    assert!(tokio::time::timeout(
        Duration::from_millis(10),
        call(
            &ctx,
            "extension:invoke-action",
            json!({"actionId":old}),
            &mut state
        )
    )
    .await
    .is_err());
    let stored: Session = serde_json::from_value(state["session"].clone()).unwrap();
    assert_eq!(stored.copy, CopyState::Pending(Action::CopyHtml));
    let before = state.clone();
    let v = call(&ctx, "extension:refresh-session", json!({}), &mut state)
        .await
        .unwrap();
    assert_eq!(state, before);
    assert!(v.to_string().contains("Retry Copy HTML"));
    assert!(call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":old}),
        &mut state
    )
    .await
    .is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn acknowledged_output_survives_later_failed_events() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    ctx.set_router(Arc::new(|_, _| Box::pin(async { Ok(json!({"ok":true})) })))
        .unwrap();
    let mut state = json!({});
    let v = preview(&ctx, &mut state, "text").await;
    call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Copy Markdown")}),
        &mut state,
    )
    .await
    .unwrap();
    let before = state.clone();
    assert!(call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":"forged"}),
        &mut state
    )
    .await
    .is_err());
    assert_eq!(state, before);
}
#[tokio::test]
async fn disposal_wipes_drafts_and_invalidates_all_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let mut state = json!({});
    let v = preview(&ctx, &mut state, "PRIVATE").await;
    call(&ctx, "extension:dispose-session", json!({}), &mut state)
        .await
        .unwrap();
    assert!(!state.to_string().contains("PRIVATE"));
    assert!(call(
        &ctx,
        "extension:invoke-action",
        json!({"actionId":action(&v,"Back to Editor")}),
        &mut state
    )
    .await
    .is_err());
}
async fn ipc(s: &Sessions, ctx: &CoreContext, op: &str, body: Value) -> Value {
    s.call(ctx, op, body).await.unwrap().unwrap()
}
async fn launch(s: &Sessions, ctx: &CoreContext) -> Value {
    ipc(s,ctx,"extension:run-command",json!({"extensionId":"com.tezbar.native.markdown-preview","commandName":"preview-markdown","argumentValues":{}})).await
}
#[tokio::test]
async fn actual_native_sessions_launch_dispatch_annotate_refresh_and_dispose() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let s = Sessions::default();
    let v = launch(&s, &ctx).await;
    assert_eq!(v["extensionId"], "com.tezbar.native.markdown-preview");
    assert!(v["sessionId"]
        .as_str()
        .unwrap()
        .starts_with("native-extension:"));
    let body = json!({"sessionId":v["sessionId"],"actionId":action(&v,"Preview Markdown"),"formValues":{"markdown":"# actual NativeSessions"}});
    let v = ipc(&s, &ctx, "extension:invoke-action", body).await;
    assert_eq!(v["title"], "Markdown Preview");
    let fresh = ipc(
        &s,
        &ctx,
        "extension:refresh-session",
        json!({"sessionId":v["sessionId"]}),
    )
    .await;
    assert_eq!(v, fresh);
    assert_eq!(
        ipc(
            &s,
            &ctx,
            "extension:search-text-changed",
            json!({"sessionId":v["sessionId"],"searchText":"unused"})
        )
        .await["mode"],
        "unchanged"
    );
    assert_eq!(
        ipc(
            &s,
            &ctx,
            "extension:dispose-session",
            json!({"sessionId":v["sessionId"]})
        )
        .await,
        true
    );
    assert_eq!(
        ipc(
            &s,
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":v["sessionId"],"actionId":action(&v,"Back to Editor")})
        )
        .await["ok"],
        false
    );
}
#[tokio::test]
async fn native_sessions_cross_session_uninstalled_unknown_and_stale_ids_reject() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let s = Sessions::default();
    let a = launch(&s, &ctx).await;
    let b = launch(&s, &ctx).await;
    assert_eq!(ipc(&s,&ctx,"extension:invoke-action",json!({"sessionId":b["sessionId"],"actionId":action(&a,"Preview Markdown"),"formValues":{"markdown":"cross"}})).await["ok"],false);
    ctx.config.patch(&json!({"installed":false})).unwrap();
    assert_eq!(
        ipc(
            &s,
            &ctx,
            "extension:invoke-action",
            json!({"sessionId":a["sessionId"],"actionId":action(&a,"Clear")})
        )
        .await["ok"],
        false
    );
    assert_eq!(launch(&s, &ctx).await["ok"], false);
    assert_eq!(
        ipc(
            &s,
            &ctx,
            "extension:run-command",
            json!({"extensionId":"com.tezbar.native.markdown-preview","commandName":"unknown"})
        )
        .await["ok"],
        false
    );
    assert_eq!(ipc(&s,&ctx,"extension:run-command",json!({"sessionId":a["sessionId"],"extensionId":"raycast.markdown-preview","commandName":"preview-markdown"})).await["ok"],false);
    s.clear_slug("markdown-preview").await;
}
#[tokio::test]
async fn native_session_limit_clear_and_independent_drafts() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let s = Sessions::default();
    let a = launch(&s, &ctx).await;
    let b = launch(&s, &ctx).await;
    let changed=ipc(&s,&ctx,"extension:invoke-action",json!({"sessionId":a["sessionId"],"actionId":a["root"]["children"][0]["props"]["actionId"],"formValues":{"value":"only a"}})).await;
    assert!(changed["root"]["children"][0]["props"]
        .get("value")
        .is_none());
    let refreshed = ipc(
        &s,
        &ctx,
        "extension:refresh-session",
        json!({"sessionId":b["sessionId"]}),
    )
    .await;
    assert_eq!(refreshed["root"]["children"][0]["props"]["value"], "");
    for _ in 2..128 {
        assert_eq!(launch(&s, &ctx).await["ok"], true);
    }
    assert_eq!(launch(&s, &ctx).await["ok"], false);
    s.clear().await;
    assert_eq!(launch(&s, &ctx).await["ok"], true);
    s.clear().await;
}
#[tokio::test]
async fn sdk_store_process_atomic_preserves_unknowns_and_rejects_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    sdk::update_store(&ctx, "fixture", |v| {
        *v = json!({"future":{"keep":true},"count":0});
        Ok(())
    })
    .await
    .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..30 {
        let ctx = ctx.clone();
        tasks.push(tokio::spawn(async move {
            sdk::update_store(&ctx, "fixture", |v| {
                v["count"] = json!(v["count"].as_u64().unwrap() + 1);
                Ok(())
            })
            .await
            .unwrap()
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(
        sdk::read_store(&ctx, "fixture").await.unwrap(),
        json!({"future":{"keep":true},"count":30})
    );
    let path = ctx
        .paths
        .data_dir
        .join("native-extensions-v2/fixture/storage.json");
    std::fs::write(&path, b"{corrupt").unwrap();
    assert!(sdk::read_store(&ctx, "fixture").await.is_err());
    assert!(sdk::update_store(&ctx, "fixture", |_| Ok(()))
        .await
        .is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"{corrupt");
    for slug in ["../escape", "", "a/b"] {
        assert!(sdk::read_store(&ctx, slug).await.is_err());
    }
}
#[test]
fn proposed_actual_runtime_catalog_agree_on_slug_alias_commands_platforms_and_not_run() {
    use markdown_preview_beta_contract::native_core::extensions::*;
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let manifest = catalog_manifest();
    assert_eq!(canonical_id(), "com.tezbar.native.markdown-preview");
    assert_eq!(manifest["totalCommands"], 1);
    assert_eq!(manifest["status"], "complete");
    for id in [
        "com.tezbar.native.markdown-preview",
        "raycast.markdown-preview",
        "markdown-preview",
    ] {
        assert!(compiled(id, "preview-markdown"));
        assert!(!compiled(id, "wrong"));
    }
    assert!(!compiled("raycast.missing", "preview-markdown"));
    assert!(manifest["verification"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["state"] == "not-run"));
    assert_eq!(manifest["icon"], "/extensions/icons/markdown-preview.png");
    let all = catalog_list(&ctx).unwrap();
    assert_eq!(all.as_array().unwrap().len(), 12);
    assert_eq!(
        all.as_array()
            .unwrap()
            .iter()
            .filter(|v| v["id"] == canonical_id())
            .count(),
        1
    );
    assert!(!ctx.paths.data_dir.exists());
}
#[tokio::test]
async fn actual_native_sessions_serialize_copy_reject_stale_queued_action_and_heartbeat_while_busy()
{
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let count = Arc::new(AtomicUsize::new(0));
    let e = entered.clone();
    let r = release.clone();
    let c = count.clone();
    ctx.set_router(Arc::new(move |channel, args| {
        assert_eq!(channel, "clipboard:write");
        assert_eq!(args, vec![json!("text")]);
        c.fetch_add(1, Ordering::SeqCst);
        let e = e.clone();
        let r = r.clone();
        Box::pin(async move {
            e.notify_one();
            r.notified().await;
            Ok(json!({"ok":true}))
        })
    }))
    .unwrap();
    let sessions = Sessions::default();
    let v = launch(&sessions, &ctx).await;
    let v=ipc(&sessions,&ctx,"extension:invoke-action",json!({"sessionId":v["sessionId"],"actionId":action(&v,"Preview Markdown"),"formValues":{"markdown":"text"}})).await;
    let body = json!({"sessionId":v["sessionId"],"actionId":action(&v,"Copy Markdown")});
    let s = sessions.clone();
    let x = ctx.clone();
    let b = body.clone();
    let first = tokio::spawn(async move { ipc(&s, &x, "extension:invoke-action", b).await });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    assert_eq!(
        ipc(
            &sessions,
            &ctx,
            "extension:refresh-session",
            json!({"sessionId":v["sessionId"]})
        )
        .await["mode"],
        "unchanged"
    );
    let s = sessions.clone();
    let x = ctx.clone();
    let second = tokio::spawn(async move { ipc(&s, &x, "extension:invoke-action", body).await });
    release.notify_one();
    assert_eq!(first.await.unwrap()["ok"], true);
    assert_eq!(second.await.unwrap()["ok"], false);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    sessions.clear().await;
}
