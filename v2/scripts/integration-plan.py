"""Prepare reviewable registration files from pinned beta. Never modifies the host."""
from pathlib import Path
import difflib,json,subprocess
v=Path(__file__).resolve().parents[1];b=v/'fixtures/beta';out=v/'integration'
out.mkdir(exist_ok=True)
prefix='src-tauri/src/native_core/extensions/'
runtime=(b/prefix/'v2_runtime.rs').read_text()
runtime=runtime.replace('"perplexity" => Some(value),','"perplexity" | "markdown-preview" => Some(value),')
runtime=runtime.replace('        "amphetamine" => super::v2_amphetamine::commands(),','        "markdown-preview" => super::v2_markdown_preview::commands(),\n        "amphetamine" => super::v2_amphetamine::commands(),')
runtime=runtime.replace('    match slug {\n        "amphetamine" => {','    match slug {\n        "markdown-preview" => super::v2_markdown_preview::handle(ctx, command, operation, payload, state).await,\n        "amphetamine" => {')
mod=(b/prefix/'mod.rs').read_text().replace('mod v2_runtime;', '#[path = "../../../native-extensions/markdown-preview/adapter/v2_markdown_preview.rs"]\nmod v2_markdown_preview;\nmod v2_runtime;')
catalog=json.loads((b/prefix/'v2_catalog.json').read_text());assert not any(x['slug']=='markdown-preview' for x in catalog);catalog.append(json.loads((v/'manifest.json').read_text()))
runtime='// Modified from pinned beta: register Markdown Preview Native V2.\n'+runtime
mod='// Modified from pinned beta: register Markdown Preview Native V2.\n'+mod
runtime=subprocess.check_output(['rustfmt','--emit','stdout','--edition','2021'],input=runtime.encode()).decode()
catalog_text=(b/prefix/'v2_catalog.json').read_text().rstrip()
append=json.dumps(catalog[-1],indent=2)
append='\n'.join('  '+line for line in append.splitlines())
catalog_output=catalog_text[:-1].rstrip()+',\n'+append+'\n]\n'
catalog_rust='// Modified from pinned beta: update expected bundled manifest count for Markdown Preview.\n'+(b/prefix/'v2_catalog.rs').read_text().replace('assert_eq!(items.len(), 11);','assert_eq!(items.len(), 12);')
files={'v2_catalog.rs':catalog_rust,'v2_runtime.rs':runtime,'mod.rs':mod,'v2_catalog.json':catalog_output}
patch=''
for name,data in files.items():
 (out/name).write_text(data)
 patch+=''.join(difflib.unified_diff((b/prefix/name).read_text().splitlines(True),data.splitlines(True),fromfile='a/'+prefix+name,tofile='b/'+prefix+name))
(out/'registration.patch').write_text(patch)
print('Prepared pinned-beta registration patch and catalog (host untouched)')
