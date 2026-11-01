# ADR-0001: Escolha do Motor de Navegação e Integração com GTK4/libadwaita

- **Status:** Aprovado (Accepted)
- **Data:** 2026-11-01
- **Autores:** Equipe de Engenharia do BEAR Browser
- **Contexto:** Definição da base tecnológica do motor de renderização web e integração com interface gráfica nativa em Linux (GTK4 + libadwaita).

---

## 1. Contexto e Problema

O **BEAR Browser** tem como objetivo central fornecer **MÁXIMA VELOCIDADE + BAIXO CONSUMO DE MEMÓRIA** em distribuições Linux (prioritariamente Linux Mint e Ubuntu x86-64), com foco intransigente em privacidade, segurança e uma interface limpa, elegante e moderna baseada no ecossistema GNOME/libadwaita.

O motor de renderização precisa garantir:
1. Compatibilidade universal com a web moderna (HTML5, ECMAScript moderno, CSS Grid/Flexbox, WebAssembly, WebRTC, DRM/Media, WebSockets).
2. Sandbox de processos robusto e multicamada no Linux (namespaces, seccomp-bpf, isolamento de processos de renderização).
3. Capacidade de interceptação profunda de requisições de rede para bloqueio de anúncios e rastreadores antes do download de dados.
4. Integração nativa e fluida com a interface GTK4/libadwaita em servidores de exibição X11 e Wayland.
5. Manutenibilidade viável para uma equipe técnica sustentável, sem exigir a recompilação e manutenção contínua de mais de 35 milhões de linhas de código de um fork direto do Chromium.

---

## 2. Alternativas Avaliadas

### Alternativa 1: Chromium Embedded Framework (CEF) via Off-Screen Rendering (OSR)
- **Descrição:** Utilizar o CEF para gerenciar o ciclo de vida do Chromium, inicialização de processos auxiliares e sandbox, enquanto o browser repassa buffers de renderização (OSR via shared memory ou texturas) diretamente para widgets GTK4 (`GdkMemoryTexture` / `GtkGLArea`).
- **Prós:**
  - Compatibilidade 100% equivalente ao Chromium estável (Blink + V8).
  - API de controle de rede madura (`CefRequestHandler`, `CefResourceRequestHandler`) permitindo cancelamento e filtragem de requisições a nível de protocolo.
  - Sandbox multi-processos pré-configurado e mantido pelo ecossistema Chromium.
  - O modo OSR funciona de forma idêntica e sem restrições tanto sob Wayland quanto sob X11, eliminando problemas históricos de `XReparentWindow` que foram removidos do GTK4.
  - Permite desenhar overlays nativos do libadwaita (popovers de abas, cantos arredondados, menus de contexto nativos, animações) sobre o canvas web sem artefatos gráficos.
- **Contras:**
  - Binários do CEF são volumosos (~1.5 GB em desenvolvimento descompactado, ~180 MB comprimido em distribuição).
  - Necessidade de gerenciar a fila de mensagens do Chromium de forma síncrona ou em thread dedicada em harmonia com o GLib Event Loop do GTK.

### Alternativa 2: CEF em Modo Janela (Windowed com X11 XReparentWindow)
- **Descrição:** Fazer com que o CEF crie uma janela nativa X11 (`Window XID`) e tentar acoplá-la a um widget GTK.
- **Prós:** Zero cópia de buffer de pixels em X11 puro.
- **Contras:**
  - **Incompatibilidade crítica com GTK4 e Wayland:** O GTK4 removeu completamente o `GtkSocket`/`GtkPlug`. O protocolo Wayland não permite reparenting arbitrário de superfícies entre clientes não cooperantes por razões de segurança.
  - Impossibilita renderização de controles GTK4 translúcidos ou arredondados sobre a página web.

### Alternativa 3: Fork Direto do Chromium / Custom Content API Embedder
- **Descrição:** Manter um fork direto do repositório Chromium, compilando-o do zero com `depot_tools` e integrando diretamente via Content API em C++.
- **Prós:** Controle absoluto sobre flags de compilação, componentes internos e patches no Blink/V8.
- **Contras:**
  - Custo de infraestrutura proibitivo (necessita 100+ GB de disco, 32GB+ RAM, 4 a 8 horas de compilação por release).
  - Custo de rebase contínuo com os releases semanais de segurança do Chromium upstream.
  - Atraso na entrega de patches de segurança de dia zero para os usuários finais.

### Alternativa 4: WebKitGTK / WPE WebKit
- **Descrição:** Motor baseado em WebKit com integração direta e nativa no GTK4.
- **Prós:** Extremamente leve na inicialização, bindings nativos de primeira classe para GTK4.
- **Contras:**
  - Não atende ao requisito fundamental do projeto ("Base tecnológica: Chromium").
  - Menor compatibilidade com sites e web apps complexos específicos do ecossistema Chromium.
  - Menor disponibilidade de tooling e ecossistema de extensões/DevTools maduro.

---

## 3. Decisão Arquitetural

Adotamos a **Alternativa 1: CEF (Chromium Embedded Framework) com Renderização Off-Screen (OSR) e Abstração Modular em Rust**.

1. **Camada de Abstração do Motor (`bear-core` / `BrowserEngine` trait):**
   A arquitetura do BEAR isola a interface gráfica e os módulos de negócio de qualquer tipo concreto do CEF através de traits em Rust (`BrowserEngine`, `BrowserTab`, `NavigationController`, `NetworkFilter`). Isso garante que a UI e os sistemas de abas, histórico, favoritos e bloqueio de anúncios permaneçam desacoplados e testáveis unitariamente.
2. **Integração Gráfica OSR com GTK4:**
   O renderizador do CEF opera em modo OSR, notificando callbacks de pintura em threads dedicadas. Os buffers de pixels são convertidos eficientemente em texturas de exibição GTK4 (`GdkMemoryTexture` / Cairo surface / GL Texture), garantindo suporte universal para **Wayland e X11**.
3. **Loop de Eventos Integrado:**
   O loop de eventos do GTK (GLib MainContext) coordena o despacho de trabalho do CEF através de `cef::do_message_loop_work` agendado a intervalos controlados (ou via thread dedicada do CEF `multi_threaded_message_loop = false/true`), evitando bloqueio da UI e mantendo o consumo de CPU em repouso próximo de 0%.
4. **Isolamento de Processos e Sandbox:**
   Subprocessos auxiliares (`bear-linux --type=renderer`, `--type=gpu-process`, etc.) são delegados ao pipeline seguro do Chromium, mantendo a proteção seccomp e namespaces ativa.

---

## 4. Consequências

- **Positivas:**
  - Compatibilidade web completa (Chromium 156+ upstream).
  - Interface nativa Linux Mint / Ubuntu utilizando temas Libadwaita modernos, dark mode dinâmico e integração perfeita com o desktop.
  - Total suporte tanto no X11 quanto no Wayland.
  - Capacidade real de bloqueio de requisições de rede através da interceptação de URLs antes do tráfego de dados.
- **Mitigações Necessárias:**
  - O empacotamento `.deb` deve gerenciar as dependências do `libcef.so`, `icudtl.dat` e arquivos `.pak`.
  - A alocação de memória no buffer OSR deve ser otimizada (reutilização de buffers e suspensão de abas inativas) para cumprir a meta de baixo consumo de RAM.
