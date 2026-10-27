# Integrating into the pinned beta host

Reference host commit: `fd300e276356587726566026f072563eb9c34cb1`. The host checkout and profiles were read only. This deliverable has not registered, installed, rebuilt, deployed or visually tested the host.

1. Vendor this crate at **`src-tauri/native-extensions/markdown-preview`** in the deliberately chosen host checkout. Keep its Cargo.lock, provenance, notices and fixtures. Do not copy V1 into the executable runtime. In host `src-tauri/Cargo.toml` add:

   ```toml
   tezbar-native-markdown-preview = { path = "native-extensions/markdown-preview" }
   ```

2. Register the adapter in `src-tauri/src/native_core/extensions/mod.rs`:

   ```rust
   #[path = "../../../native-extensions/markdown-preview/adapter/v2_markdown_preview.rs"]
   mod v2_markdown_preview;
   ```

   The adapter imports sibling `super::v2_sdk`; the SDK, common context and NativeSessions stay unchanged.

3. Review `integration/registration.patch`. It adds the slug to `v2_runtime::slug`, maps commands to `v2_markdown_preview::commands()` and dispatches every operation to its `handle`. It appends `manifest.json` to `v2_catalog.json`. The generated files are based on the pinned host, **not instructions to overwrite a newer catalog**. Merge only the Markdown registration into the current host. Adapt the host's catalog unit-test count from 11 to 12 (or its current real total + 1); assert canonical ID, alias, command inventory and compiled coverage.

4. Copy `assets/markdown-preview.png` unchanged to **`src/renderer/public/extensions/icons/markdown-preview.png`**. The manifest uses `/extensions/icons/markdown-preview.png`; `v2_catalog::store_item` derives `iconUrl` from that path. Keep the original V1 package and icon available beside V2. Do not add unsupported root `author` to schema2; credit xjo_nd in description and notices.

5. Review/apply the required `integration/markdown-copy-ack.patch` to the host-owned Markdown code-block copy feedback. The pinned component labels null as “Copied”; the patch requires a positive, non-contradictory object reply from `window.tezbar.clipboardWriteText`. The exact beta fixture remains unchanged, and the renderer harness tests both the observed bug and 13 patched reply cases. This is a host integration prerequisite; the Rust adapter’s own copy actions already enforce strict acknowledgement.

6. Rebuild the Rust host and frontend with the host's documented toolchain and commands. JSON or a `native-v2.json` marker cannot install executable Rust. Only then use the existing canonical NativeV2 installer; do not launch a JS worker as a fallback for the canonical ID.

7. Verify `v2_runtime::supports("com.tezbar.native.markdown-preview", "preview-markdown")` and the `raycast.markdown-preview` alias, catalog command count/coverage, both declared OS platforms and both `not-run` verification entries. The standalone harness compiles this proposed registration into the actual pinned runtime and catalog; other adapters and installation policy are explicitly stubbed there. Test V1/V2 coexistence in a rebuilt host before changing either verification state.

## Wire and effects

NativeSessions operations are exactly `extension:run-command`, `extension:invoke-action`, `extension:search-text-changed`, `extension:refresh-session`, `extension:load-more`, `extension:dispose-session`. Launch arguments would be in `argumentValues`, preferences in `payload.preferences`; this source declares neither and ignores both. Textarea callback is `formValues.value`; submission is the complete `formValues.markdown` map. Stable field ID is `markdown`. Callback responses omit the controlled value so a delayed response cannot overwrite a newer queued local draft. Preview/Back/Clear rotate session capabilities; malformed requests consume none. Source text and computed HTML are captured in adapter state and never taken from forged action `content`, `output` or `url` fields.

Refresh renders only already-committed IDs and does not mutate state. Search/load-more return unchanged without IDs or side effects. Tokens combine a random process nonce and monotonic counter, so discarded state clones cannot mint IDs that later revive. NativeSessions serializes committed state and cancels in-flight work on disposal/clear. Copy intent and fresh explicit retry capabilities are checkpointed before awaiting the host; acknowledgements and uncertain stages survive later failures.

Only `v2_sdk::copy` → `clipboard:write` with `[text]` is used. A reply must be an object with `ok: true`, `error` absent or null, and `success` absent or true. Null, booleans, contradictions, malformed and transport errors never mean success. Host errors are replaced by a generic, private-data-safe uncertain-result message in Detail metadata. Copy buttons are normal NativeV2 action descriptors; no arbitrary hotkeys are claimed.

Other pinned route evidence was inspected: `clipboard:read []` and `core:platform:selected-text []` return strings; SDK open uses `shell:open [target]`; `core:platform:paste-text [text]` already writes, hides, restores focus, waits and pastes, while `core:platform:paste []` uses prepared clipboard. None belongs to this source and none is called. **Pinned `window:hide []` hides and then returns null**, which this strict acknowledgement contract rejects; a future adapter using it must first obtain a real host object acknowledgement. Do not normalize arbitrary null into success or issue a second paste. There is no genuine HUD/toast/pop-to-root SDK API needed or invented here.

## Contract harness boundaries

`harness-template/` is materialized under a disposable `src-tauri` layout. Exact beta `common.rs`, `v2_sdk.rs` and `native.rs` are copied; catalog code has only its expected bundled count updated to 12, the proposed runtime registration/catalog are copied, and the production adapter is loaded through the exact sibling path above. The engine itself is a Cargo path dependency at the stable vendoring path. Tauri AppHandle/Emitter, installation/registry policy, other native adapters, host OS routes and presentation-only UI dependencies are labeled stubs.

The full upstream ExtensionsCore unit suite needs the full host; it is not claimed. For library-test compilation only, the unchanged NativeSessions module and its public harness facade are excluded with `cfg(not(test))`; integration tests compile and exercise the real module. The unchanged SDK/common unit tests can run independently with their documented filters. No production fixture bytes are patched to make these tests pass.
