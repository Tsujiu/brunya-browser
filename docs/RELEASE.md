# Manual de Release, Empacotamento e Distribuição — Brunya Browser

Este documento descreve os procedimentos formais de engenharia para gerar releases, assinar pacotes, criar o repositório APT e auditar a segurança dos artefatos do Brunya Browser.

---

## 1. Pipeline de Empacotamento Debian (.deb)

O script `scripts/build-deb.sh` automatiza todo o processo de construção em ambiente limpo:

1. **Compilação Otimizada:** Executa `cargo build --workspace --release --bin brunya-linux` com Thin LTO e nível 3 de otimização.
2. **Estagiamento:** Cria a estrutura de diretórios em conformidade com o padrão Debian FHS:
   - `/usr/bin/brunya-browser`: Binário stripped.
   - `/usr/share/applications/brunya-browser.desktop`: Metadados do desktop e suporte a ações rápidas.
   - `/usr/share/icons/hicolor/scalable/apps/brunya-browser.svg`: Ícone vetorial oficial.
   - `/usr/share/doc/brunya-browser/copyright`: aviso de direitos reservados, conforme LICENSE.
   - `/usr/share/doc/brunya-browser/changelog.Debian.gz`: Histórico de versões comprimido.
   - `/usr/share/man/man1/brunya-browser.1.gz`: Manual do usuário do sistema.
3. **Scripts de Controle (`DEBIAN/`):**
   - `control`: Dependências estritas (`libgtk-4-1`, `libadwaita-1-0`, `libsqlite3-0`, `libc6`).
   - `postinst`: Registra alternativas `x-www-browser` e atualiza caches de desktop e ícones.
   - `postrm`: Remove alternativas e limpa caches na desinstalação.
   - `md5sums`: Hashes para verificação de integridade pós-instalação (`debsums`).
4. **Geração do Arquivo:** Utiliza `dpkg-deb --build --root-owner-group` gerando o pacote em `dist/brunya-browser_<versão>_amd64.deb`.

---

## 2. Infraestrutura de Repositório APT Assinado

O script `scripts/setup-apt-repo.sh` estrutura o repositório APT em conformidade com o padrão Debian Repository Format:

- Gera índice `Packages` e `Packages.gz` contendo hashes MD5, SHA-1 e SHA-256 de todos os pacotes.
- Gera `Release` com metadados de validade temporal (`Valid-Until: 90 days`), prevenindo ataques de replay.
- Assina com GPG:
  - `Release.gpg`: Assinatura destacada.
  - `InRelease`: Assinatura em texto claro (clear-signed) exigida por versões modernas do APT.
- Exporta a chave pública oficial em formatos `.gpg` e `.asc` para distribuição aos clientes.

---

## 3. Geração de Software Bill of Materials (SBOM)

O script `scripts/generate-sbom.sh` analisa o arquivo `Cargo.lock` e inventaria todas as dependências transitivas e diretas em conformidade com a especificação **CycloneDX 1.5 JSON**:
- Saída: `dist/brunya-browser-sbom.cdx.json`
- Cada componente inclui nome, versão, PURL (Package URL), escopo e tipo.
- Permite auditoria automatizada em ferramentas de análise de vulnerabilidade (ex: Dependency-Track, Trivy).

---

## 4. Integração Contínua e Entrega Contínua (CI/CD)

- `.github/workflows/ci.yml`:
  - Executado em cada pull request e push para a branch `main`.
  - Etapas: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace --verbose`, `brunya-linux --benchmark-full`, geração de `.deb` e SBOM.
- `.github/workflows/release.yml`:
  - Disparado automaticamente na criação de tags semânticas (ex: `git tag v0.3.0 && git push origin v0.3.0`).
  - Publica o pacote `.deb`, o SBOM e os hashes `SHA256SUMS` no GitHub Releases.
