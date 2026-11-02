# Registro de Mudanças (Changelog) — Brunya Browser

Todas as alterações notáveis neste projeto serão documentadas neste arquivo.
O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

---

## [0.4.0-alpha] - 2026-11-01

### Adicionado
- **Fase 3 Concluída:** Desempenho e Otimizações de Memória.
- **Detector de Pressão de Memória Física:** Leitura direta de `/proc/meminfo` e PSI (`SystemMemorySnapshot`) com classificação em Normal, Moderada e Crítica.
- **Suspensão Emergencial Adaptativa:** Descarte em massa instantâneo de abas inativas sob pressão crítica de RAM com retenção leve de metadados (`DiscardedTabState`).
- **Detecção em Tempo de Execução de Extensões de CPU:** Módulo `CpuCapabilities` com detecção dinâmica de SSE4.2, AES-NI, AVX2, BMI2 e RDRAND, preservando total compatibilidade universal do binário x86-64 sem flags agressivas de compilação.
- **Suíte de Benchmark Automatizada Integrada:** Flag CLI `--benchmark-full` medindo com precisão microssegundos a carga de subsistemas, consumo de RAM (1/10/30 abas), rendimento do adblock (>710.000 req/s) e latência de restauração (3 µs).
- **Otimizações de Compilação Release:** Perfil otimizado com opt-level=3, Thin LTO, codegen-units=1 e strip de debuginfo.

## [0.3.0-alpha] - 2026-11-01

### Adicionado
- **Fase 2 Concluída:** Privacidade e Bloqueio Avançado.
- **Motor Adblock com Parsing Sintático ABP:** Suporte completo a âncoras de domínio (`||`), âncoras de início/fim (`|`), regras de exceção (`@@`), regras cosméticas (`##`), tipos de recursos (`$script`, `$image`) e restrições de terceiros (`$third-party`).
- **Atualização Atômica de Listas de Filtros:** Módulo `FilterListManager` com verificação de integridade criptográfica SHA-256 (`ring::digest`) e suporte a rollback seguro (`.bak`).
- **Estatísticas Granulares de Privacidade:** Categorização automática de ameaças (Publicidade, Analítico/Telemetria, Widgets Sociais, Fingerprinting e Mineração) com visualização por aba.
- **Painel do Escudo de Privacidade Aprimorado:** Seletor de Modo Balanceado vs Modo Rigoroso e listagem dos domínios bloqueados na aba ativa.
- **Diálogo de Permissões por Site:** Ancorado ao botão de segurança da barra de URL, permitindo alternar permissões de Câmera, Microfone, Localização e Notificações por origem.
- **Configurações de DNS Seguro (DoH):** Suporte a DNS-over-HTTPS com provedor Quad9 padrão (sem logs) e Cloudflare.
- **Modo Privado Completo:** Janelas privadas isoladas sem persistência em disco de histórico/cookies e página explicativa dos limites de privacidade (`brunya://private`).

## [0.2.0-alpha] - 2026-11-01

### Adicionado
- **Fase 1 Concluída:** Navegador funcional para uso diário no Linux.
- **Gerenciador de Downloads:** Pipeline de streaming com monitoramento de progresso, persistência SQLite (migração de esquema v2), botões de abrir arquivo e pasta (`gio::AppInfo`) e atalho `Ctrl+J`.
- **Sistema de Favoritos Integrado:** Botão de estrela na barra de endereço, atalho `Ctrl+D`, página interna `brunya://bookmarks` e remoção com um clique.
- **Histórico com Pesquisa em Tempo Real:** Filtragem dinâmica por termo em títulos e URLs em `brunya://history`, com limpeza total e persistência de timestamps.
- **Navegação Avançada de Abas:** Gesto de clique direito com menu de contexto (Duplicar, Fixar/Desafixar, Fechar), alternância rápida via `Ctrl+Tab` e `Ctrl+Shift+Tab`.
- **Barra de Localizar na Página (Find in Page):** Ativação por `Ctrl+F` e fechamento com `Esc`, com interface compacta no topo da página.
- **Interface e Navegação Expandida:** Botões de downloads e favoritos no HeaderBar, atalhos convencionais e tratamento completo de erros.

## [0.1.0-alpha] - 2026-11-01

### Adicionado
- **Fase 0 Concluída:** Prova de conceito e arquitetura fundamental do Brunya Browser.
- **Estrutura Modular:** Workspace Cargo com 9 componentes (`brunya-linux`, `brunya-core`, `brunya-i18n`, `brunya-storage`, `brunya-adblock`, `brunya-privacy`, `brunya-tabs`, `brunya-settings`, `brunya-diagnostics`).
- **ADR-0001:** Decisão arquitetural documentada adotando Chromium/CEF via Off-Screen Rendering (OSR) desacoplado em Rust e GTK4/libadwaita.
- **Interface Nativa GTK4/libadwaita:** Janela moderna com HeaderBar, controles de navegação, barra de abas nativa com suporte a fechar/trocar abas e botão de nova aba.
- **Navegação Web Real:** Carregamento de páginas ao vivo pela rede (ex: `https://kernel.org`), com extração de título HTML e renderização limpa no viewport.
- **Páginas Internas Nativas:** Implementadas `brunya://newtab`, `brunya://settings`, `brunya://history` e `brunya://about`.
- **Escudo de Privacidade Brunya (Adblock Integrado):** Interceptação pré-download de requisições de rede com bloqueio ativo de rastreadores, telemetria invasiva e redes de anúncios.
- **Internacionalização (i18n):** Suporte nativo completo a Português do Brasil (`pt-BR`) como referência e Inglês (`en`), com paridade de catálogos validada por testes.
- **Armazenamento Seguro em SQLite:** Tabela de histórico e favoritos com migrações de esquema versionadas (v1) e conformidade ACID.
- **Economizador Inteligente de Memória:** Estrutura e timer de suspensão automática de abas inativas em segundo plano para preservação de RAM.
- **Verificação de Sandbox Linux:** Validação de suporte a unprivileged user namespaces e seccomp-bpf no kernel do sistema operacional.
- **Medição de Desempenho e Benchmarks:** Carga de módulos em 1,20 ms e consumo inicial de memória RSS de apenas 22 a 24 MB.
