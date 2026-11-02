# Registro de Mudanças (Changelog) — BEAR Browser

Todas as alterações notáveis neste projeto serão documentadas neste arquivo.
O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

---

## [0.2.0-alpha] - 2026-11-01

### Adicionado
- **Fase 1 Concluída:** Navegador funcional para uso diário no Linux.
- **Gerenciador de Downloads:** Pipeline de streaming com monitoramento de progresso, persistência SQLite (migração de esquema v2), botões de abrir arquivo e pasta (`gio::AppInfo`) e atalho `Ctrl+J`.
- **Sistema de Favoritos Integrado:** Botão de estrela na barra de endereço, atalho `Ctrl+D`, página interna `bear://bookmarks` e remoção com um clique.
- **Histórico com Pesquisa em Tempo Real:** Filtragem dinâmica por termo em títulos e URLs em `bear://history`, com limpeza total e persistência de timestamps.
- **Navegação Avançada de Abas:** Gesto de clique direito com menu de contexto (Duplicar, Fixar/Desafixar, Fechar), alternância rápida via `Ctrl+Tab` e `Ctrl+Shift+Tab`.
- **Barra de Localizar na Página (Find in Page):** Ativação por `Ctrl+F` e fechamento com `Esc`, com interface compacta no topo da página.
- **Interface e Navegação Expandida:** Botões de downloads e favoritos no HeaderBar, atalhos convencionais e tratamento completo de erros.

## [0.1.0-alpha] - 2026-11-01

### Adicionado
- **Fase 0 Concluída:** Prova de conceito e arquitetura fundamental do BEAR Browser.
- **Estrutura Modular:** Workspace Cargo com 9 componentes (`bear-linux`, `bear-core`, `bear-i18n`, `bear-storage`, `bear-adblock`, `bear-privacy`, `bear-tabs`, `bear-settings`, `bear-diagnostics`).
- **ADR-0001:** Decisão arquitetural documentada adotando Chromium/CEF via Off-Screen Rendering (OSR) desacoplado em Rust e GTK4/libadwaita.
- **Interface Nativa GTK4/libadwaita:** Janela moderna com HeaderBar, controles de navegação, barra de abas nativa com suporte a fechar/trocar abas e botão de nova aba.
- **Navegação Web Real:** Carregamento de páginas ao vivo pela rede (ex: `https://kernel.org`), com extração de título HTML e renderização limpa no viewport.
- **Páginas Internas Nativas:** Implementadas `bear://newtab`, `bear://settings`, `bear://history` e `bear://about`.
- **Escudo de Privacidade BEAR (Adblock Integrado):** Interceptação pré-download de requisições de rede com bloqueio ativo de rastreadores, telemetria invasiva e redes de anúncios.
- **Internacionalização (i18n):** Suporte nativo completo a Português do Brasil (`pt-BR`) como referência e Inglês (`en`), com paridade de catálogos validada por testes.
- **Armazenamento Seguro em SQLite:** Tabela de histórico e favoritos com migrações de esquema versionadas (v1) e conformidade ACID.
- **Economizador Inteligente de Memória:** Estrutura e timer de suspensão automática de abas inativas em segundo plano para preservação de RAM.
- **Verificação de Sandbox Linux:** Validação de suporte a unprivileged user namespaces e seccomp-bpf no kernel do sistema operacional.
- **Medição de Desempenho e Benchmarks:** Carga de módulos em 1,20 ms e consumo inicial de memória RSS de apenas 22 a 24 MB.
