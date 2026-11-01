# Guia de Compilação e Reprodução — BEAR Browser

Este documento detalha o procedimento exato e reproduzível para compilar, testar e executar o BEAR Browser a partir do código-fonte em distribuições Linux (com foco em **Linux Mint 22** e **Ubuntu 24.04 LTS**).

---

## 1. Requisitos de Sistema

- **Arquitetura:** `x86_64` (compatibilidade padrão com instruções base x86-64-v1/v2, sem exigência de AVX2 no build universal).
- **Memória RAM:** Mínimo de 4 GB de RAM (recomendado 8 GB ou mais para compilações rápidas).
- **Armazenamento:** Mínimo de 5 GB livres em disco (para artefatos de build do Rust e bibliotecas de suporte).
- **Sistema Operacional:** Linux Mint 22.x (Zena/Zara) ou Ubuntu 24.04 LTS (Noble Numbat).

---

## 2. Dependências do Sistema

Instale os pacotes de desenvolvimento necessários via APT:

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libgtk-4-dev \
    libadwaita-1-dev \
    libsqlite3-dev \
    curl \
    git
```

---

## 3. Instalação da Toolchain Rust

Recomenda-se a versão estável mais recente do Rust através do `rustup`:

```bash
# Instalar rustup (caso não esteja instalado)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Verificar versões
rustc --version # Requerido 1.85+
cargo --version
```

---

## 4. Compilação do Workspace

### Compilação de Desenvolvimento (Debug)
```bash
cargo build --workspace
```

### Compilação Otimizada de Produção (Release)
O perfil de release está configurado no `Cargo.toml` raiz com:
- `opt-level = 3`
- `lto = "thin"`
- `codegen-units = 1`
- `strip = "debuginfo"`

```bash
cargo build --workspace --release
```

Os binários finais serão gerados em:
`target/release/bear-linux`

---

## 5. Execução de Testes Automatizados

Para rodar todos os testes unitários e de integração de todos os crates:

```bash
cargo test --workspace
```

Para rodar testes de um módulo específico (exemplo, adblock):
```bash
cargo test -p bear-adblock
```

---

## 6. Execução do Navegador

### Executar a aplicação principal
```bash
cargo run -p bear-linux
```

### Abrir com URLs específicas na inicialização
```bash
cargo run -p bear-linux -- "https://kernel.org" "https://linuxmint.com"
```

### Executar benchmark de inicialização e memória
```bash
cargo run -p bear-linux -- --benchmark
```

### Verificar o suporte do sandbox no Linux
```bash
cargo run -p bear-linux -- --check-sandbox
```
