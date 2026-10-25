"""Audit actual tarball contents and run self-contained extracted engine/adapter offline."""
from pathlib import Path
import subprocess,tarfile,json,hashlib,shutil
v=Path(__file__).resolve().parents[1]
def run(args,cwd=v):subprocess.run(list(map(str,args)),cwd=cwd,check=True)
run(['cargo','package','--locked','--offline','--allow-dirty','--manifest-path',v/'Cargo.toml'])
archive=v/'target/package/tezbar-native-markdown-preview-0.1.0.crate'
out=v/'.scratch/extracted';out.mkdir(parents=True,exist_ok=True)
with tarfile.open(archive) as tar:
 names=tar.getnames()
 assert not any('/node_modules/' in p or '/.scratch/' in p or '/target/' in p for p in names)
 # Cargo-generated archive, still enforce safe relative members.
 assert all(not Path(p).is_absolute() and '..' not in Path(p).parts for p in names)
 tar.extractall(out)
root=out/'tezbar-native-markdown-preview-0.1.0'
for file in json.loads((v/'PROVENANCE.json').read_text())['files']:
 data=(root/file['path']).read_bytes();assert len(data)==file['bytes'] and hashlib.sha256(data).hexdigest()==file['sha256'],file['path']
for path in ['LICENSE','adapter/v2_markdown_preview.rs','harness-template/Cargo.toml.in','harness-template/Cargo.lock','fixtures/libraries/dompurify/dist/purify.cjs.js','fixtures/beta/src-tauri/src/native_core/common.rs']:
 assert (root/path).is_file(),path
for toolchain in ['stable','1.85.0']:
 run(['cargo','+'+toolchain,'test','--locked','--offline','--manifest-path',root/'Cargo.toml'])
manifest=root/'.scratch/check/src-tauri/Cargo.toml'
run(['python3',root/'scripts/materialize-harness.py'])
for toolchain in ['stable','1.85.0']:
 run(['cargo','+'+toolchain,'test','--locked','--offline','--manifest-path',manifest])
 run(['cargo','+'+toolchain,'clippy','--locked','--offline','--all-targets','--manifest-path',manifest,'--','-D','warnings'])
print('Cargo tarball: every provenance path + nested templates included; extracted engine/adapter tests and Clippy pass offline')
