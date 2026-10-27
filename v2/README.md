# Markdown Preview — Rust Native V2

Standalone Rust engine and concrete compiled-in beta adapter for **Preview Markdown**, beside the unchanged V1 extension. Original extension and icon: **xjo_nd**, Raycast `6734d85992290066fd31a7f5bee3cca3d3545a1a`, no listed contributors. Own behavior source: `almatkai/markdown-preview` at `3cd8b99d5919ca6098db3edc08ea1a9f59ec0600`.

V2 starts with an empty Markdown Content textarea. Preview submits the complete text, displays its Markdown and UTF-16 character / ECMAScript whitespace word / LF line statistics, and exposes Back to Editor, Copy Markdown and Copy HTML. Back preserves the exact draft. Clear resets it. Blank submissions stay in the editor. The source has **no preferences, arguments, selection/clipboard initialization, paste, open, file export or persistent state**; its `storeValue={false}` is intentional. README claims about paste and shortcuts do not add source behavior.

Extension execution is Rust only. `adapter/v2_markdown_preview.rs` uses the pinned beta SDK; the existing React Form/Detail/Markdown renderer consumes normal NativeV2 wire values. Node and all JavaScript fixtures are validation only. JSON cannot install executable Rust: [host registration and rebuild](INTEGRATION.md) are required before this appears as a working Native V2 command.

## Fidelity and limits

- The supported adaptation is **Form → Detail → Form**, with preserved edit/back drafts. Beta Form does not implement the source textarea's `enableMarkdown` editing aid. No live formatted editor or side-by-side preview is claimed. The actual source itself pushes a separate preview on submission.
- Beta's actual `Markdown.tsx` uses Marked 18, DOMPurify and syntax highlighting; Copy HTML uses the Rust Marked-14-compatible GFM converter. The executed source oracle covers 184 inputs; this is bounded parity evidence, not a proof against the entire Marked/CommonMark language. [Validation](VALIDATION.md) states the exact tested features and visual/desktop gaps.
- Original arbitrary action hotkeys are not advertised because pinned Surface does not dispatch them. The host's existing submit, action panel, Back and Detail primary-action controls remain available.
- Extension copy actions require a strict positive object acknowledgement. The pinned host’s separate code-block copy button also needs the supplied [strict acknowledgement patch](integration/markdown-copy-ack.patch) during integration. Failed, malformed or lost responses show an uncertain outcome and fresh explicit retry actions. No automatic repeat, window hiding, synthetic HUD/toast or forged event-provided copy output is used.
- Limits: 1 MiB UTF-8 input, 8 MiB generated HTML, 128 parser nesting levels. Errors are explicit; no truncation. Invalid calls leave legitimate capabilities and drafts intact. Closing a session wipes its draft. No unknown legacy storage migration is attempted.

The original icon is byte-exact at `assets/markdown-preview.png`: 53,005 bytes, SHA-256 `3dbd6f9010889aae1ee8b5d768aed2d6c92b808e1d0bc668a9d330d187aff40e`. Publish it at `/extensions/icons/markdown-preview.png` by copying it into the host's `src/renderer/public/extensions/icons/markdown-preview.png`.

## Reproduce

From the repository root:

```sh
cargo test --locked --manifest-path v2/Cargo.toml
python3 v2/scripts/prepare-oracle.py
node v2/scripts/source-oracle.cjs
npm ci --ignore-scripts --no-audit --no-fund --prefix v2/validation
node v2/scripts/schema-and-integrity.cjs
node v2/scripts/renderer-contract.cjs
python3 v2/scripts/materialize-harness.py
cargo test --locked --manifest-path v2/.scratch/check/src-tauri/Cargo.toml
python3 v2/scripts/check-all.py
```

The V1 disposable install needs `--legacy-peer-deps` because its existing locked `@types/react` / `@types/react-dom` peer declarations conflict. The preparation script supplies that flag without altering the source lock. `check-all.py` executes stable/MSRV formatting, tests, strict Clippy and Windows GNU checks; first install both Rust toolchains and Windows GNU standard libraries. All test filesystem writes stay inside disposable directories. No system clipboard, selection, browser, window, focus or paste effects run.

[COPYRIGHT / notices](NOTICES.md), [provenance and license hashes](PROVENANCE.json), [validation results and gaps](VALIDATION.md), [integration](INTEGRATION.md). Platform verification remains `not-run` for both macOS and Windows until a rebuilt host receives real desktop QA.
