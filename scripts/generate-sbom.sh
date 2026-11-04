#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== BRUNYA BROWSER — GERADOR DE SBOM (CYCLONEDX JSON) ==="
cd "$ROOT_DIR"

OUTPUT_DIR="$ROOT_DIR/dist"
mkdir -p "$OUTPUT_DIR"
SBOM_FILE="$OUTPUT_DIR/brunya-browser-sbom.cdx.json"

python3 -c "
import json, os, re, datetime

root_dir = '$ROOT_DIR'
lock_file = os.path.join(root_dir, 'Cargo.lock')

components = []

if os.path.exists(lock_file):
    with open(lock_file, 'r', encoding='utf-8') as f:
        content = f.read()

    packages = re.findall(r'\[\[package\]\]\nname = \"([^\"]+)\"\nversion = \"([^\"]+)\"(?:\nsource = \"([^\"]+)\")?', content)
    for name, version, source in packages:
        comp = {
            'type': 'library',
            'name': name,
            'version': version,
            'purl': f'pkg:cargo/{name}@{version}',
            'scope': 'required'
        }
        if name.startswith('brunya-'):
            comp['type'] = 'application' if name == 'brunya-linux' else 'library'
            comp['supplier'] = {'name': 'Brunya Browser Team', 'url': ['https://brunyabrowser.org']}
            comp['licenses'] = [{'license': {'name': 'LicenseRef-Brunya-All-Rights-Reserved'}}]
        components.append(comp)

sbom = {
    'bomFormat': 'CycloneDX',
    'specVersion': '1.5',
    'serialNumber': f'urn:uuid:brunya-browser-sbom-{datetime.datetime.now(datetime.timezone.utc).strftime(\"%Y%m%d%H%M%S\")}',
    'version': 1,
    'metadata': {
        'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'tools': [
            {'vendor': 'Brunya Browser Project', 'name': 'brunya-sbom-generator', 'version': '1.0.0'}
        ],
        'component': {
            'type': 'application',
            'name': 'brunya-browser',
            'version': '0.3.0',
            'description': 'Navegador web rápido, leve e privado para Linux',
            'licenses': [{'license': {'name': 'LicenseRef-Brunya-All-Rights-Reserved'}}],
            'purl': 'pkg:deb/ubuntu/brunya-browser@0.3.0?arch=amd64'
        }
    },
    'components': components
}

with open('$SBOM_FILE', 'w', encoding='utf-8') as f:
    json.dump(sbom, f, indent=2, ensure_ascii=False)

print(f'• SBOM gerado com sucesso: {len(components)} componentes inventariados')
print('• Arquivo de saída: $SBOM_FILE')
"

ls -lh "$SBOM_FILE"
