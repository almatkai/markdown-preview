//! Contract harness. AppHandle/Emitter, registry and compiled dispatch are labeled stubs.
//! common.rs, v2_sdk.rs and native.rs are byte-exact beta files, not reimplementations.
extern crate self as tauri;
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, value: serde_json::Value) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}
pub mod native_core;
