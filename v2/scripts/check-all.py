"""Locked engine/production-adapter gates. Does not access host checkout or real OS routes."""
from pathlib import Path
import subprocess
v=Path(__file__).resolve().parents[1]
def run(args):
 print('+ '+' '.join(map(str,args)),flush=True);subprocess.run(list(map(str,args)),cwd=v,check=True)
run(['python3',v/'scripts/materialize-harness.py'])
manifest=v/'.scratch/check/src-tauri/Cargo.toml'
run(['cargo','fmt','--all','--','--check'])
run(['rustfmt','--check','--edition','2021','--config','skip_children=true',v/'adapter/v2_markdown_preview.rs',v/'harness-template/src/lib.rs',v/'harness-template/src/native_core/mod.rs',v/'harness-template/src/native_core/extensions/mod.rs',v/'harness-template/tests/adapter.rs'])
for toolchain in ['stable','1.85.0']:
 for cargo_manifest in [v/'Cargo.toml',manifest]:
  for op in ['test','clippy']:
   args=['cargo','+'+toolchain,op,'--locked','--all-targets','--manifest-path',cargo_manifest]
   if op=='clippy':args+=['--','-D','warnings']
   run(args)
  for op in ['check','clippy']:
   args=['cargo','+'+toolchain,op,'--locked','--all-targets','--manifest-path',cargo_manifest,'--target','x86_64-pc-windows-gnu']
   if op=='clippy':args+=['--','-D','warnings']
   run(args)
# Execute actual unchanged common/SDK tests, without upstream whole-host catalog/ExtensionsCore tests.
for selected in ['common::','v2_sdk::']:
 run(['cargo','test','--locked','--manifest-path',manifest,'--lib',selected])
