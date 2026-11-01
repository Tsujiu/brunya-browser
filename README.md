# 🐻 BEAR Browser

> **Rápido por engenharia, leve por arquitetura, privado por padrão.**

O **BEAR Browser** é um navegador desktop moderno, open source e voltado para privacidade, projetado em **Rust** com interface nativa em **GTK4 + libadwaita** e motor baseado no ecossistema **Chromium / CEF (Chromium Embedded Framework)**.

Seu foco central é atingir **MÁXIMA VELOCIDADE + BAIXO CONSUMO DE MEMÓRIA RAM**, sem concessões à segurança, com bloqueador de anúncios integrado a nível de protocolo de rede e suporte oficial a **Português brasileiro (pt-BR)** e **Inglês (en)**.

---

## 🚀 Destaques do Projeto

- **Núcleo em Rust:** Código próprio seguro em memória, livre de vazamentos e com alta performance concorrente.
- **Interface Nativa Linux:** Construída com **GTK4** e **libadwaita 1.5+**, integrando-se perfeitamente aos desktops Linux Mint (Cinnamon), GNOME e Ubuntu. Suporte nativo a **Wayland** e **X11**.
- **Motor Web de Alta Performance:** Compatibilidade com Chromium 156+ upstream (Blink, V8, WebAssembly, WebRTC) através de arquitetura Off-Screen Rendering (OSR) desacoplada.
- **Escudo de Privacidade BEAR Integrado:** Bloqueio de anúncios, rastreadores e scripts de telemetria invasivos antes do download dos dados de rede, sem depender de extensões de terceiros.
- **Economizador Inteligente de Memória:** Suspensão automática de abas inativas em segundo plano para liberar memória RAM sob pressão do sistema.
- **Privacidade Absoluta:** Sem telemetria oculta, sem exigência de conta de usuário, com particionamento de cookies e navegação privada verdadeira.
- **Internacionalização desde o Início:** Arquitetura centralizada de tradução com pt-BR como idioma de referência e inglês oficial secundário.
- **Armazenamento Seguro:** Histórico, favoritos e preferências gerenciados em SQLite com migrações versionadas (ACID).

---

## 🛠️ Requisitos e Ambiente

- **Sistema Operacional:** Linux Mint 22+, Ubuntu 24.04 LTS (ou distribuições Debian-based equivalentes).
- **Arquitetura:** Linux x86-64 (suporte universal sem instruções AVX obrigatórias).
- **Compilador Rust:** Rustc 1.85+ (testado e validado com Rustc 1.99.1).
- **Bibliotecas:** `libgtk-4-dev` (4.14+), `libadwaita-1-dev` (1.5+), `libsqlite3-dev` (3.45+).

---

## 📦 Como Compilar e Executar

Consulte o guia completo em [docs/BUILD.md](docs/BUILD.md).

### 1. Clonar o repositório
```bash
git clone https://github.com/bear-browser/bear-browser.git
cd bear-browser
```

### 2. Instalar dependências no Linux Mint / Ubuntu
```bash
sudo apt update
sudo apt install -y build-essential libgtk-4-dev libadwaita-1-dev libsqlite3-dev clang
```

### 3. Executar a suíte de testes
```bash
cargo test --workspace
```

### 4. Executar em modo desenvolvimento
```bash
cargo run -p bear-linux
```

### 5. Abrir uma URL diretamente
```bash
cargo run -p bear-linux -- "https://kernel.org"
```

---

## 📊 Estrutura do Workspace

```text
├── apps/
│   └── bear-linux/          # Aplicação GTK4/libadwaita executável no Linux
├── crates/
│   ├── bear-core/           # Tipos de dados, URLs, identificadores e traits de motor
│   ├── bear-i18n/           # Sistema de internacionalização (pt-BR e en)
│   ├── bear-storage/        # Persistência SQLite (histórico, favoritos, permissões)
│   ├── bear-adblock/        # Motor de bloqueio de anúncios e rastreadores
│   ├── bear-privacy/        # Gestão de permissões, cookies e modo privado
│   ├── bear-tabs/           # Gerenciador de abas e suspensão inteligente de memória
│   ├── bear-settings/       # Configurações do usuário e preferências de sistema
│   └── bear-diagnostics/    # Telemetria local, medição de RSS e benchmarks de inicialização
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

## 📜 Licença

Código próprio distribuído sob a Licença **Apache-2.0**.
Consulte o arquivo [LICENSE](LICENSE) para mais detalhes.
Third-party notices e licenças de dependências disponíveis em [docs/LICENSES-THIRD-PARTY.md](docs/LICENSES-THIRD-PARTY.md).
