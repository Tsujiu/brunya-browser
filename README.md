# 🌐 Brunya Browser

> **Rápido por engenharia, leve por arquitetura, privado por padrão.**

O **Brunya Browser** é um navegador desktop voltado para privacidade, projetado em **Rust** com interface nativa em **GTK4 + libadwaita** e motor WebKitGTK. O código é público para consulta. Não é código aberto: copiar, redistribuir, comercializar ou criar um projeto derivado depende da minuta em [LICENSE](LICENSE).

Seu foco central é atingir **MÁXIMA VELOCIDADE + BAIXO CONSUMO DE MEMÓRIA RAM**, sem concessões à segurança, com bloqueador de anúncios integrado a nível de protocolo de rede e suporte oficial a **Português brasileiro (pt-BR)** e **Inglês (en)**.

---

## 🚀 Destaques do Projeto

- **Núcleo em Rust:** Código próprio seguro em memória, livre de vazamentos e com alta performance concorrente.
- **Interface Nativa Linux:** Construída com **GTK4** e **libadwaita 1.5+**, integrando-se perfeitamente aos desktops Linux Mint (Cinnamon), GNOME e Ubuntu. Suporte nativo a **Wayland** e **X11**.
- **Motor Web de Alta Performance:** Compatibilidade com Chromium 156+ upstream (Blink, V8, WebAssembly, WebRTC) através de arquitetura Off-Screen Rendering (OSR) desacoplada.
- **Escudo de Privacidade Brunya Integrado:** Bloqueio de anúncios, rastreadores e scripts de telemetria invasivos antes do download dos dados de rede, sem depender de extensões de terceiros.
- **Economizador Inteligente de Memória:** Suspensão automática de abas inativas em segundo plano para liberar memória RAM sob pressão do sistema.
- **Privacidade Absoluta:** Sem telemetria oculta, sem exigência de conta de usuário, com particionamento de cookies e navegação privada verdadeira.
- **Internacionalização desde o Início:** Arquitetura centralizada de tradução com pt-BR como idioma de referência e inglês oficial secundário.
- **Armazenamento Seguro:** Histórico, favoritos e preferências gerenciados em SQLite com migrações versionadas (ACID).

---

## 🛠️ Requisitos e Ambiente

- **Sistema Operacional:** Linux Mint 22+, Ubuntu 24.04 LTS (ou distribuições Debian-based equivalentes).
- **Arquitetura:** Linux x86-64 (suporte universal sem instruções AVX obrigatórias).
- **Compilador Rust:** Rustc 1.85+ (testado e validado com Rustc 1.99.1).
- **Bibliotecas:** `libgtk-4-dev` (4.14+), `libadwaita-1-dev` (1.5+), `libwebkitgtk-6.0-dev`, `libsoup-3.0-dev`, `libsqlite3-dev` (3.45+).

---

## 📦 Como Compilar e Executar

Consulte o guia completo em [docs/BUILD.md](docs/BUILD.md).

### 1. Clonar o repositório
```bash
git clone https://github.com/Tsujiu/brunya-browser.git
cd brunya-browser
```

### 2. Instalar dependências no Linux Mint / Ubuntu
```bash
sudo apt update
sudo apt install -y build-essential libgtk-4-dev libadwaita-1-dev libwebkitgtk-6.0-dev libsoup-3.0-dev libjavascriptcoregtk-6.0-dev libsqlite3-dev clang
```

### 3. Executar a suíte de testes
```bash
cargo test --workspace
```

### 4. Executar em modo desenvolvimento
```bash
cargo run -p brunya-linux
```

### 5. Abrir uma URL diretamente
```bash
cargo run -p brunya-linux -- "https://kernel.org"
```

---

## 📊 Estrutura do Workspace

```text
├── apps/
│   └── brunya-linux/        # Aplicação GTK4/libadwaita executável no Linux
├── crates/
│   ├── brunya-core/         # Tipos de dados, URLs, identificadores e traits de motor
│   ├── brunya-i18n/         # Sistema de internacionalização (pt-BR e en)
│   ├── brunya-storage/      # Persistência SQLite (histórico, favoritos, permissões)
│   ├── brunya-adblock/      # Motor de bloqueio de anúncios e rastreadores
│   ├── brunya-privacy/      # Gestão de permissões, cookies e modo privado
│   ├── brunya-tabs/         # Gerenciador de abas e suspensão inteligente de memória
│   ├── brunya-settings/     # Configurações do usuário e preferências de sistema
│   └── brunya-diagnostics/  # Telemetria local, medição de RSS e benchmarks de inicialização
├── locales/
│   ├── pt-BR/               # Catálogos em português brasileiro (referência)
│   └── en/                  # Catálogos em inglês
└── docs/
    ├── ARCHITECTURE.md      # Arquitetura e decisões técnicas
    ├── BENCHMARKS.md        # Medições e metodologia de performance
    ├── BUILD.md             # Guia de compilação reproduzível
    ├── adr/                 # Architecture Decision Records (ADRs)
    └── screenshots/         # Capturas reais da interface gráfica
```

---

## Direitos autorais

Copyright 2026 Brunya Browser Team. Todos os direitos reservados.

O repositório público permite ver o código, compilar para avaliação pessoal e não comercial, e abrir um Pull Request de volta para este projeto. Não autoriza espelho, fork independente, redistribuição nem uso comercial. A minuta está em [LICENSE](LICENSE). Avisos de terceiros estão em [NOTICE](NOTICE).

Contribuição só entra com o aceite registrado da minuta em [CLA.md](CLA.md). As regras estão em [CONTRIBUTING.md](CONTRIBUTING.md). Vulnerabilidade segue [SECURITY.md](SECURITY.md).

Esta documentação não é parecer jurídico. A minuta pede revisão por advogado.
