//! Production adapter: compiled inside native_core::extensions with the unchanged beta SDK.
use super::v2_sdk as sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::{json, Value};
use tezbar_native_markdown_preview::{acknowledged, Outcome, Session, COMMAND};
pub fn commands() -> &'static [&'static str] {
    &[COMMAND]
}
pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    if command != COMMAND {
        return Err("Unknown Markdown Preview command".into());
    }
    if operation == "extension:run-command" {
        if state.get("session").is_some() {
            return Err("Markdown Preview is already running".into());
        }
        // The source declares neither arguments nor preferences and starts empty.
        let session = Session::default();
        let view = session.view();
        *state = json!({"session":session});
        return Ok(view);
    }
    let mut session: Session = serde_json::from_value(
        state
            .get("session")
            .cloned()
            .ok_or("Markdown Preview session is not initialized")?,
    )
    .map_err(|_| "Invalid Markdown Preview session state")?;
    if operation == "extension:dispose-session" {
        session.dispose();
        *state = json!({"session":session});
        return Ok(sdk::no_view("Markdown Preview closed"));
    }
    if session.disposed {
        return Err("Markdown Preview session has ended".into());
    }
    match operation {
        // These calls never mint capabilities. NativeSessions can stage/discard clones.
        "extension:refresh-session" => Ok(session.view()),
        "extension:search-text-changed" | "extension:load-more" => Ok(sdk::unchanged()),
        "extension:invoke-action" => {
            let result = session.invoke(payload);
            // Preserve drafts even when conversion fails; malformed events do not mutate.
            *state = json!({"session":session});
            match result? {
                Outcome::Copy(text) => {
                    // Intent + fresh explicit retry capabilities are committed BEFORE await.
                    let result = sdk::copy(ctx, &text).await;
                    let ok = acknowledged(&result);
                    session.finish_copy(ok);
                    *state = json!({"session":session});
                    // A view is the supported result presentation; do not expose host errors,
                    // synthesize HUD/toast success, hide the window, or auto-repeat a copy.
                    Ok(session.view())
                }
                Outcome::View => Ok(session.view()),
                Outcome::Unchanged => Ok(sdk::unchanged()),
            }
        }
        _ => Err("Unsupported Markdown Preview operation".into()),
    }
}
