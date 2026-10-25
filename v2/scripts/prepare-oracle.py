from pathlib import Path
import shutil,subprocess
v=Path(__file__).resolve().parents[1]; out=v/'.scratch/v1'
out.mkdir(parents=True,exist_ok=True)
for p in (v/'fixtures/own').rglob('*'):
 if p.is_file():
  dest=out/p.relative_to(v/'fixtures/own');dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,dest)
subprocess.run(['npm','ci','--ignore-scripts','--legacy-peer-deps','--no-audit','--no-fund'],cwd=out,check=True)
