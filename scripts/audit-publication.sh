#!/bin/bash
# Auditoria local antes de publicar. Falha se achar segredo evidente,
# arquivo que não deve ir ao Git, ou texto que volte a chamar o projeto
# de código aberto.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

fail=0
note() { printf 'ERRO: %s\n' "$1"; fail=1; }

while IFS= read -r path; do
  case "$path" in
    *.env|*.env.*|*.pem|*.key|*.p12|*.pfx|id_rsa|id_rsa.pub|*.sqlite|*.sqlite3|*.dump|*.sql.gz)
      note "arquivo rastreado que não deve ser publicado: $path"
      ;;
  esac
done < <(git ls-files)

if git grep -n -I -E 'BEGIN (RSA |OPENSSH |EC )?PRIVATE KEY|AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}' -- . ':!scripts/audit-publication.sh' ':!Cargo.lock' >/tmp/brunya-audit-secrets.txt 2>/dev/null; then
  note "padrão de segredo no conteúdo rastreado"
  cat /tmp/brunya-audit-secrets.txt
fi

if git grep -n -I -E 'código aberto|open source|Apache-2\.0' -- README.md CONTRIBUTING.md apps/brunya-linux/src/main.rs locales/pt-BR/settings.json locales/en/settings.json Cargo.toml \
  | grep -v -E 'Não é código aberto|não é código aberto|not open source' \
  >/tmp/brunya-audit-license.txt 2>/dev/null; then
  if [ -s /tmp/brunya-audit-license.txt ]; then
    note "texto ainda apresenta o Brunya como código aberto ou Apache-2.0"
    cat /tmp/brunya-audit-license.txt
  fi
fi

grep -q 'Todos os direitos reservados' LICENSE || note "LICENSE não reserva os direitos"
grep -q 'CLA-BRUNYA-2026-11-02' CLA.md || note "CLA.md não tem o identificador vigente"
grep -q 'CLA-BRUNYA-2026-11-02' .github/workflows/cla.yml || note "o fluxo de CLA não usa o identificador vigente"

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "Auditoria de publicação: nenhum bloqueio encontrado."
