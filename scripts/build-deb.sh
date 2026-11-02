#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== BRUNYA BROWSER — GERADOR DE PACOTE DEBIAN (.deb) ==="
cd "$ROOT_DIR"

# 1. Obter versão do workspace
VERSION=$(grep -m1 '^version' Cargo.toml | awk -F '"' '{print $2}')
ARCH="amd64"
DEB_NAME="brunya-browser_${VERSION}_${ARCH}.deb"
OUTPUT_DIR="$ROOT_DIR/dist"
STAGING_DIR="$ROOT_DIR/target/staging-deb"

echo "• Versão detectada: $VERSION"
echo "• Arquitetura: $ARCH"
echo "• Pacote de saída: $DEB_NAME"

# 2. Compilar binário release otimizado
echo "• Compilando binário release otimizado (opt-level=3, LTO)..."
cargo build --workspace --release --bin brunya-linux

# 3. Preparar diretório de estagiamento
rm -rf "$STAGING_DIR"
mkdir -p \
    "$STAGING_DIR/DEBIAN" \
    "$STAGING_DIR/usr/bin" \
    "$STAGING_DIR/usr/share/applications" \
    "$STAGING_DIR/usr/share/icons/hicolor/scalable/apps" \
    "$STAGING_DIR/usr/share/doc/brunya-browser" \
    "$STAGING_DIR/usr/share/man/man1" \
    "$OUTPUT_DIR"

# 4. Copiar executável e aplicar strip
echo "• Instalando executável em /usr/bin/brunya-browser..."
cp "$ROOT_DIR/target/release/brunya-linux" "$STAGING_DIR/usr/bin/brunya-browser"
strip --strip-unneeded "$STAGING_DIR/usr/bin/brunya-browser"
chmod 755 "$STAGING_DIR/usr/bin/brunya-browser"

# 5. Instalar arquivo .desktop e ícone SVG
echo "• Instalando integração com desktop e ícones..."
cp "$ROOT_DIR/packaging/debian/brunya-browser.desktop" "$STAGING_DIR/usr/share/applications/"
chmod 644 "$STAGING_DIR/usr/share/applications/brunya-browser.desktop"

cp "$ROOT_DIR/assets/icons/hicolor/scalable/apps/brunya-browser.svg" \
   "$STAGING_DIR/usr/share/icons/hicolor/scalable/apps/brunya-browser.svg"
chmod 644 "$STAGING_DIR/usr/share/icons/hicolor/scalable/apps/brunya-browser.svg"

# Instalar ícones dos motores de busca e escudos
mkdir -p "$STAGING_DIR/usr/share/brunya-browser/icons/engines"
if [ -d "$ROOT_DIR/assets/icons/engines" ]; then
    cp -r "$ROOT_DIR/assets/icons/engines/"* "$STAGING_DIR/usr/share/brunya-browser/icons/engines/"
fi
cp "$ROOT_DIR/assets/icons/brunya-shield"*.svg "$STAGING_DIR/usr/share/brunya-browser/icons/" || true
find "$STAGING_DIR" -type d -exec chmod 755 {} +
find "$STAGING_DIR" -type f -exec chmod 644 {} +
chmod 755 "$STAGING_DIR/usr/bin/brunya-browser"

# 6. Gerar documentação Debian e Copyright
echo "• Gerando documentação de copyright e changelog Debian..."
cat << 'EOF' > "$STAGING_DIR/usr/share/doc/brunya-browser/copyright"
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: Brunya Browser
Source: https://github.com/brunya-browser/brunya-browser

Files: *
Copyright: 2026 Brunya Browser Team <engineering@brunyabrowser.org>
License: Apache-2.0
 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 .
 http://www.apache.org/licenses/LICENSE-2.0
EOF
chmod 644 "$STAGING_DIR/usr/share/doc/brunya-browser/copyright"

cat << EOF | gzip -9c > "$STAGING_DIR/usr/share/doc/brunya-browser/changelog.Debian.gz"
brunya-browser ($VERSION) stable; urgency=medium

  * Release oficial do Brunya Browser $VERSION para Linux Mint e Ubuntu.
  * Suporte nativo a GTK4 e Libadwaita com arquitetura Chromium/CEF.
  * Bloqueador de anúncios integrado e proteções de privacidade ativas.

 -- Brunya Browser Team <engineering@brunyabrowser.org>  $(date -R)
EOF
chmod 644 "$STAGING_DIR/usr/share/doc/brunya-browser/changelog.Debian.gz"

# 7. Gerar manual (manpage)
cat << EOF | gzip -9c > "$STAGING_DIR/usr/share/man/man1/brunya-browser.1.gz"
.TH BRUNYA-BROWSER 1 "$(date +'%Y-%m-%d')" "brunya-browser $VERSION" "User Commands"
.SH NAME
brunya-browser \- Navegador web rápido, leve e privado para Linux
.SH SYNOPSIS
.B brunya-browser
[\fIOPTIONS\fR] [\fIURL\fR...]
.SH DESCRIPTION
Brunya Browser é um navegador desktop moderno baseado em Rust e Chromium,
desenvolvido com foco em velocidade, baixo consumo de RAM e privacidade.
.SH OPTIONS
.TP
\fB\-v\fR, \fB\-\-version\fR
Exibe a versão do navegador e do motor de renderização.
.TP
\fB\-h\fR, \fB\-\-help\fR
Exibe mensagem de ajuda e opções disponíveis.
.TP
\fB\-\-check\-sandbox\fR
Verifica os recursos de sandbox e isolamento no kernel Linux.
.TP
\fB\-\-benchmark\-full\fR
Executa a suíte de testes de desempenho de inicialização e memória.
.SH AUTHOR
Brunya Browser Team <engineering@brunyabrowser.org>
EOF
chmod 644 "$STAGING_DIR/usr/share/man/man1/brunya-browser.1.gz"

# 8. Copiar scripts de controle Debian
echo "• Configurando metadados DEBIAN (control, postinst, postrm)..."
cp "$ROOT_DIR/packaging/debian/control" "$STAGING_DIR/DEBIAN/control"
# Atualizar versão no control dinamicamente
sed -i "s/^Version:.*/Version: $VERSION/" "$STAGING_DIR/DEBIAN/control"

# Calcular tamanho instalado em kB
INSTALLED_SIZE=$(du -sk "$STAGING_DIR" | cut -f1)
echo "Installed-Size: $INSTALLED_SIZE" >> "$STAGING_DIR/DEBIAN/control"

cp "$ROOT_DIR/packaging/debian/postinst" "$STAGING_DIR/DEBIAN/postinst"
cp "$ROOT_DIR/packaging/debian/postrm" "$STAGING_DIR/DEBIAN/postrm"
chmod 755 "$STAGING_DIR/DEBIAN/postinst" "$STAGING_DIR/DEBIAN/postrm"

# 9. Gerar hashes MD5 dos arquivos instalados
(
    cd "$STAGING_DIR"
    find usr -type f -exec md5sum {} + > DEBIAN/md5sums
    chmod 644 DEBIAN/md5sums
)

# 10. Empacotar com dpkg-deb
echo "• Empacotando com dpkg-deb..."
dpkg-deb --build --root-owner-group "$STAGING_DIR" "$OUTPUT_DIR/$DEB_NAME"

echo "================================================================================"
echo "   PACOTE .DEB GERADO COM SUCESSO: $OUTPUT_DIR/$DEB_NAME"
echo "================================================================================"
ls -lh "$OUTPUT_DIR/$DEB_NAME"
dpkg-deb -I "$OUTPUT_DIR/$DEB_NAME"
dpkg-deb -c "$OUTPUT_DIR/$DEB_NAME"
