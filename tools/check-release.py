"""Validate only public release metadata; never read signing secrets."""
from pathlib import Path
import json,re,os
root=Path(__file__).resolve().parents[1]
package=json.loads((root/'package.json').read_text(encoding='utf-8'))
config=json.loads((root/'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
cargo=re.search(r'^version\s*=\s*"([^"]+)"',(root/'src-tauri/Cargo.toml').read_text(encoding='utf-8'),re.M)[1]
assert package['version']==config['version']==cargo,'Inconsistent app versions'
tag=os.environ.get('GITHUB_REF','')
if tag.startswith('refs/tags/'):
    assert tag=='refs/tags/v'+cargo,'Tag and app version differ'
updater=config['plugins']['updater']
assert updater['endpoints']==['https://github.com/moidefr/photo-editor/releases/latest/download/latest.json']
assert updater['pubkey'] and config['bundle']['createUpdaterArtifacts'] is True
assert not updater.get('dangerousInsecureTransportProtocol')
for source in config['bundle']['resources']:
    assert (root/'src-tauri'/source).exists(),source
print('Release metadata verified:',cargo,'windows-x86_64')
