from pathlib import Path
import subprocess, urllib.request, hashlib, json, shutil
root=Path(__file__).resolve().parents[2]; v=root/'v2'
own='3cd8b99d5919ca6098db3edc08ea1a9f59ec0600'; beta='fd300e276356587726566026f072563eb9c34cb1'; original='6734d85992290066fd31a7f5bee3cca3d3545a1a'; host='/Users/almat/Desktop/code/Tezbar APP/Tezbar'
records=[]
def save(path,data,source):
 out=v/path; out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
 records.append({'path':str(path),'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'source':source})
for path in subprocess.check_output(['git','ls-tree','-r','--name-only',own],cwd=root).decode().splitlines():
 save(Path('fixtures/own')/path,subprocess.check_output(['git','show',own+':'+path],cwd=root),f'own:{own}:{path}')
paths=['docs/NATIVE_V2_FORMAT.md','schemas/extension-v2.schema.json','src-tauri/src/native_core/common.rs','src-tauri/src/native_core/platform.rs','src-tauri/src/native_core/launcher/clipboard.rs','src-tauri/src/native_core/extensions/v2_sdk.rs','src-tauri/src/native_core/extensions/v2_runtime.rs','src-tauri/src/native_core/extensions/native.rs','src-tauri/src/native_core/extensions/v2_catalog.rs','src-tauri/src/native_core/extensions/v2_catalog.json','src-tauri/src/native_core/extensions/mod.rs','src/renderer/ui/Markdown.tsx','src/shared/extensionRuntime.ts','src/renderer/src/raycast-api/index.tsx','src/renderer/src/raycast-api/form-runtime.tsx','src/renderer/src/raycast-api/detail-runtime.tsx','src/renderer/src/raycast-api/action-runtime-overlay.tsx','src/renderer/src/raycast-api/action-runtime-registry.tsx','src/renderer/src/raycast-api/list-runtime.tsx','src/renderer/src/raycast-api/action-runtime.tsx','src-tauri/Cargo.toml','package.json','pnpm-lock.yaml','LICENSE']
for path in paths:
 try: data=subprocess.check_output(['git','-C',host,'show',beta+':'+path],stderr=subprocess.PIPE)
 except subprocess.CalledProcessError as e:print('GAP',path,e.stderr.decode().strip());continue
 save(Path('fixtures/beta')/(path+'.fixture' if path=='src-tauri/Cargo.toml' else path),data,f'beta:{beta}:{path}')
for path in ['extensions/markdown-preview/package.json','extensions/markdown-preview/src/preview-markdown.tsx','extensions/markdown-preview/assets/md-preview-icon.png','LICENSE']:
 data=urllib.request.urlopen(f'https://raw.githubusercontent.com/raycast/extensions/{original}/{path}',timeout=30).read()
 save(Path('fixtures/original')/path,data,f'original:{original}:{path}')
icon=(v/'fixtures/original/extensions/markdown-preview/assets/md-preview-icon.png').read_bytes()
assert len(icon)==53005 and hashlib.sha256(icon).hexdigest()=='3dbd6f9010889aae1ee8b5d768aed2d6c92b808e1d0bc668a9d330d187aff40e'
assert icon==(root/'assets/md-preview-icon.png').read_bytes()
save(Path('assets/markdown-preview.png'),icon,'original icon, exact bytes')
(v/'PROVENANCE.json').write_text(json.dumps({'ownSha':own,'betaSha':beta,'originalSha':original,'files':records},indent=2)+'\n')
