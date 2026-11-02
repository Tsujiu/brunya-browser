#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== BRUNYA BROWSER — GERADOR DE REPOSITÓRIO APT ASSINADO ==="
cd "$ROOT_DIR"

DIST_DIR="$ROOT_DIR/dist"
REPO_DIR="$DIST_DIR/apt-repo"
DEB_FILE=$(ls -1 "$DIST_DIR"/brunya-browser_*_amd64.deb 2>/dev/null | head -n 1 || true)

if [ -z "$DEB_FILE" ]; then
    echo "Erro: Pacote .deb não encontrado em $DIST_DIR. Execute ./scripts/build-deb.sh primeiro."
    exit 1
fi

echo "• Pacote base: $DEB_FILE"

# 1. Estruturar diretórios do repositório APT
ARCH_DIR="$REPO_DIR/dists/stable/main/binary-amd64"
POOL_DIR="$REPO_DIR/pool/main/b/brunya-browser"
rm -rf "$REPO_DIR"
mkdir -p "$ARCH_DIR" "$POOL_DIR" "$REPO_DIR/keys"

# 2. Copiar .deb para a pool
cp "$DEB_FILE" "$POOL_DIR/"
DEB_FILENAME=$(basename "$DEB_FILE")

# 3. Gerar arquivo Packages
echo "• Gerando índice Packages..."
FILE_SIZE=$(stat -c%s "$DEB_FILE")
MD5_SUM=$(md5sum "$DEB_FILE" | awk '{print $1}')
SHA1_SUM=$(sha1sum "$DEB_FILE" | awk '{print $1}')
SHA256_SUM=$(sha256sum "$DEB_FILE" | awk '{print $1}')

# Extrair campos do control
dpkg-deb -f "$DEB_FILE" > "$ARCH_DIR/Packages"
cat << EOF >> "$ARCH_DIR/Packages"
Filename: pool/main/b/brunya-browser/$DEB_FILENAME
Size: $FILE_SIZE
MD5sum: $MD5_SUM
SHA1: $SHA1_SUM
SHA256: $SHA256_SUM

EOF

gzip -9c "$ARCH_DIR/Packages" > "$ARCH_DIR/Packages.gz"

# 4. Gerar arquivo Release da distribuição
echo "• Gerando Release da distribuição..."
RELEASE_FILE="$REPO_DIR/dists/stable/Release"
cat << EOF > "$RELEASE_FILE"
Origin: Brunya Browser Project
Label: Brunya Browser
Suite: stable
Codename: stable
Version: 1.0
Architectures: amd64
Components: main
Description: Repositório oficial do Brunya Browser para Linux Mint e Ubuntu
Date: $(date -Ru)
Valid-Until: $(date -Ru -d "+90 days")
MD5Sum:
 $(md5sum "$ARCH_DIR/Packages" | awk '{print $1}') $(stat -c%s "$ARCH_DIR/Packages") main/binary-amd64/Packages
 $(md5sum "$ARCH_DIR/Packages.gz" | awk '{print $1}') $(stat -c%s "$ARCH_DIR/Packages.gz") main/binary-amd64/Packages.gz
SHA256:
 $(sha256sum "$ARCH_DIR/Packages" | awk '{print $1}') $(stat -c%s "$ARCH_DIR/Packages") main/binary-amd64/Packages
 $(sha256sum "$ARCH_DIR/Packages.gz" | awk '{print $1}') $(stat -c%s "$ARCH_DIR/Packages.gz") main/binary-amd64/Packages.gz
EOF

# 5. Gerar chave GPG do repositório para assinatura (se não existir chave dedicada)
GNUPGHOME="$REPO_DIR/.gnupg"
mkdir -p "$GNUPGHOME"
chmod 700 "$GNUPGHOME"

cat << 'EOF' > "$GNUPGHOME/keygen.conf"
%echo Gerando chave GPG para o repositório Brunya Browser...
Key-Type: RSA
Key-Length: 4096
Name-Real: Brunya Browser Archive Automatic Signing Key
Name-Email: 142347573+Tsujiu@users.noreply.github.com
Expire-Date: 2y
%no-protection
%commit
%echo Chave GPG gerada com sucesso.
EOF

gpg --homedir "$GNUPGHOME" --batch --generate-key "$GNUPGHOME/keygen.conf"

KEY_ID=$(gpg --homedir "$GNUPGHOME" --list-keys --with-colons | grep '^pub' | cut -d: -f5)
echo "• Chave GPG gerada: $KEY_ID"

# 6. Assinar o arquivo Release (Release.gpg destacado e InRelease clear-sign)
echo "• Assinando metadados APT (Release.gpg e InRelease)..."
gpg --homedir "$GNUPGHOME" --default-key "$KEY_ID" -abs -o "$REPO_DIR/dists/stable/Release.gpg" "$RELEASE_FILE"
gpg --homedir "$GNUPGHOME" --default-key "$KEY_ID" --clearsign -o "$REPO_DIR/dists/stable/InRelease" "$RELEASE_FILE"

# 7. Exportar chave pública para /keys/brunya.gpg
gpg --homedir "$GNUPGHOME" --export "$KEY_ID" > "$REPO_DIR/keys/brunya-archive-keyring.gpg"
gpg --homedir "$GNUPGHOME" --armor --export "$KEY_ID" > "$REPO_DIR/keys/brunya-archive-keyring.asc"

# Limpar GNUPGHOME temporário mantendo apenas a chave pública
rm -rf "$GNUPGHOME"

echo "================================================================================"
echo "   REPOSITÓRIO APT ASSINADO CRIADO EM: $REPO_DIR"
echo "================================================================================"
find "$REPO_DIR" -maxdepth 4 -not -path '*/.*'
