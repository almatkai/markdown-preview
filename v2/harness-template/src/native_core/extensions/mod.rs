// Upstream native unit tests require full ExtensionsCore; integration tests exercise this unchanged module.
#[cfg(not(test))]
mod native;
#[path = "../../../native-extensions/markdown-preview/adapter/v2_markdown_preview.rs"]
pub mod v2_markdown_preview;
pub mod v2_sdk;
#[cfg(not(test))]
use crate::native_core::common::HandlerResult;
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::{json, Value};
#[cfg(not(test))]
use std::sync::{Arc, Mutex};
/// INSTALL/REGISTRY STUB: per-context test policy, no profile or installed database writes.
pub mod registry {
    use super::*;
    pub fn installed_command(ctx: &CoreContext, id: &str, command: &str) -> Option<Value> {
        let config = ctx.config.read().ok()?;
        if config["installed"] == false {
            return None;
        }
        (matches!(
            id,
            "com.tezbar.native.markdown-preview" | "raycast.markdown-preview"
        ) && command == "preview-markdown")
            .then(|| json!({"preferences":config["preferences"]}))
    }
    pub fn preferences(extension: &Value, _: Option<&str>) -> CoreResult {
        Ok(extension["preferences"]
            .as_object()
            .map_or(json!({}), |v| json!(v)))
    }
}
// The pinned runtime is copied with ONLY the proposed Markdown registration.
mod v2_catalog;
mod v2_runtime;
// OTHER-ADAPTER STUBS: no existing adapter implementation/coverage is claimed here.
macro_rules! unavailable_adapter {
    ($name:ident) => {
        mod $name {
            pub fn commands() -> &'static [&'static str] {
                &[]
            }
            pub async fn handle(
                _: &crate::native_core::common::CoreContext,
                _: &str,
                _: &str,
                _: &serde_json::Value,
                _: &mut serde_json::Value,
            ) -> crate::native_core::common::CoreResult {
                Err("Other adapter stub".into())
            }
        }
    };
}
unavailable_adapter!(v2_amphetamine);
unavailable_adapter!(v2_audio_device);
unavailable_adapter!(v2_cleanshotx);
unavailable_adapter!(v2_color_picker);
unavailable_adapter!(v2_gif_search);
unavailable_adapter!(v2_google_calendar);
unavailable_adapter!(v2_google_search);
unavailable_adapter!(v2_port_manager);
unavailable_adapter!(v2_speedtest);
unavailable_adapter!(v2_timers);
unavailable_adapter!(v2_perplexity);
pub fn catalog_manifest() -> Value {
    v2_catalog::manifest("markdown-preview").unwrap()
}
pub fn catalog_list(ctx: &CoreContext) -> CoreResult {
    v2_catalog::cli_list(ctx)
}
pub fn catalog_install(ctx: &CoreContext, id: &str) -> CoreResult {
    v2_catalog::cli_install(ctx, id)
}
pub fn compiled(id: &str, command: &str) -> bool {
    v2_runtime::supports(id, command)
}
pub fn canonical_id() -> String {
    v2_runtime::canonical_id("markdown-preview")
}
#[cfg(not(test))]
#[derive(Clone, Default)]
pub struct Sessions(native::NativeSessions, Arc<Mutex<()>>);
#[cfg(not(test))]
impl Sessions {
    pub async fn call(&self, ctx: &CoreContext, channel: &str, body: Value) -> HandlerResult {
        self.0.handle(ctx, &self.1, channel, &[body]).await
    }
    pub async fn clear(&self) {
        self.0.clear().await;
    }
    pub async fn clear_slug(&self, slug: &str) {
        self.0.clear_slug(slug).await;
    }
}

pub async fn dispatch(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    v2_runtime::handle(ctx, "markdown-preview", command, operation, payload, state).await
}
