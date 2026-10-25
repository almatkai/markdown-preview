from pathlib import Path
import subprocess,json,hashlib
v=Path(__file__).resolve().parents[1]
inventory=[]
for manifest in [v/'Cargo.toml',v/'.scratch/check/src-tauri/Cargo.toml']:
 metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--manifest-path',str(manifest)]))
 for package in metadata['packages']:
  if package['source'] is None:continue
  ident=package['name']+'-'+package['version']
  if any(x['id']==ident for x in inventory):continue
  root=Path(package['manifest_path']).parent;files=[]
  candidates=[x for x in root.iterdir() if x.name.lower().startswith(('license','copying','unlicense'))]
  for s in sorted(set(candidates)):
   if not s.is_file():continue
   dest=v/'licenses/rust'/ident/s.name;dest.parent.mkdir(parents=True,exist_ok=True);data=s.read_bytes();dest.write_bytes(data)
   files.append({'path':str(dest.relative_to(v)),'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)})
  if not files:
   for filename in ['README.md','AUTHORS','Cargo.toml.orig']:
    source=root/filename
    if source.is_file():
     dest=v/'licenses/rust'/ident/filename;dest.parent.mkdir(parents=True,exist_ok=True);data=source.read_bytes();dest.write_bytes(data)
     files.append({'path':str(dest.relative_to(v)),'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'evidenceOnly':True})
  inventory.append({'id':ident,'declaredSpdx':package['license'],'source':package['source'],'files':files})
(v/'licenses/RUST-INVENTORY.json').write_text(json.dumps(inventory,indent=2)+'\n')
p=json.loads((v/'PROVENANCE.json').read_text())
for package in inventory:
 for file in package['files']:
  if not any(x['path']==file['path'] for x in p['files']):p['files'].append({**file,'source':'Cargo.lock crate license: '+package['id']})
(v/'PROVENANCE.json').write_text(json.dumps(p,indent=2)+'\n')
print('Inventoried',len(inventory),'locked Rust dependencies')
