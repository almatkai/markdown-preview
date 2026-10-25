"""Materialize nested test crates from included templates (Cargo package omits nested manifests)."""
from pathlib import Path
import shutil,sys
v=Path(__file__).resolve().parents[1]
out=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else v/'.scratch/check/src-tauri'
out.mkdir(parents=True,exist_ok=True)
for p in (v/'harness-template').rglob('*'):
 if p.is_file():
  name=p.relative_to(v/'harness-template'); dest=out/name
  if name.name=='Cargo.toml.in':dest=dest.with_name('Cargo.toml')
  dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,dest)
for source,dest in [('src-tauri/src/native_core/common.rs','src/native_core/common.rs'),('src-tauri/src/native_core/extensions/v2_sdk.rs','src/native_core/extensions/v2_sdk.rs'),('src-tauri/src/native_core/extensions/native.rs','src/native_core/extensions/native.rs')]:
 shutil.copyfile(v/'fixtures/beta'/source,out/dest)
for name in ['v2_runtime.rs','v2_catalog.json','v2_catalog.rs']:
 shutil.copyfile(v/'integration'/name,out/'src/native_core/extensions'/name)
engine=out/'native-extensions/markdown-preview'
for name in ['src','adapter','assets','fixtures','tests','licenses']:
 shutil.copytree(v/name,engine/name,dirs_exist_ok=True)
for name in ['Cargo.toml','Cargo.lock','README.md','LICENSE']:shutil.copyfile(v/name,engine/name)
print(out/'Cargo.toml')
