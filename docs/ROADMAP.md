# Roteiro de Desenvolvimento (Roadmap) — Brunya Browser

O desenvolvimento do Brunya Browser é conduzido em etapas progressivas, guiadas por critérios objetivos de aceitação e comprovação de métricas.

---

## 🟢 FASE 0 — Auditoria, Arquitetura e Prova de Conceito (CONCLUÍDA)
- [x] Inspecionar o ambiente de desenvolvimento (Linux Mint 22 / Ubuntu 24.04 x86-64).
- [x] Elaborar o relatório técnico comparativo e o ADR-0001 (CEF vs Content API vs GTK4).
- [x] Estruturar o repositório modular e o workspace Cargo (`apps/brunya-linux` + 8 crates).
- [x] Implementar a janela nativa GTK4/libadwaita com controles de navegação e abas.
- [x] Implementar navegação real na web (carregamento ao vivo de `https://kernel.org`).
- [x] Integrar motor de internacionalização com paridade de catálogos pt-BR e en.
- [x] Integrar persistência SQLite ACID para histórico e favoritos com migrações versionadas.
- [x] Implementar o Escudo de Privacidade com bloqueador de anúncios e interceptação de rede.
- [x] Validar sandbox seccomp e namespaces no kernel Linux.
- [x] Atingir benchmark de memória RSS inicial (<25 MB) e tempo de prontidão (<150 ms).

---

## 🟢 FASE 1 — Navegador Funcional e Experiência Diária (CONCLUÍDA)
- [x] Gerenciador de downloads com suporte a streaming, pausas, cancelamentos e retomadas.
- [x] Página interna `brunya://downloads` com botões de abrir arquivo e pasta no sistema operacional.
- [x] Suporte a histórico expansível com pesquisa textual em tempo real e ordenação cronológica.
- [x] Sistema de favoritos com botão dedicado, atalho `Ctrl+D` e página `brunya://bookmarks`.
- [x] Integração com barra de busca na página (`Ctrl+F`) com escape rápido (`Esc`).
- [x] Gestos de abas: menu de contexto de clique direito (duplicar, fixar/desafixar, fechar).
- [x] Atalhos de navegação de abas (`Ctrl+Tab`, `Ctrl+Shift+Tab`, `Ctrl+T`, `Ctrl+W`, `Ctrl+Shift+T`).
- [x] Persistência em SQLite com migração de esquema v2.

---

## 🟢 FASE 2 — Privacidade Avançada e Bloqueio Granular (CONCLUÍDA)
- [x] Motor adblock com parsing completo de sintaxe ABP/EasyList (`||`, `@@`, `##`, `$third-party`, `$script`).
- [x] Atualização atômica de listas de filtros com verificação SHA-256 e rollback de emergência.
- [x] Rastreamento granular de bloqueios por aba com categorização de rastreadores.
- [x] Modos de proteção: Balanceado (padrão) e Rigoroso (máxima proteção).
- [x] Diálogo de permissões de site por origem (câmera, microfone, localização, notificações) integrado ao cadeado de segurança.
- [x] Configurações de DNS Seguro (DNS-over-HTTPS / DoH) com provedores Quad9 e Cloudflare.
- [x] Janela de navegação privada (`brunya://private`) com garantia de não persistência de histórico ou dados em disco.

---

## 🟡 FASE 3 — Desempenho e Otimizações de Memória (PRÓXIMA ETAPA)
- [ ] Otimizações de pipeline com EGL / DMA-BUF OSR zero-copy entre GPU do Chromium e GSK do GTK4.
- [ ] Ajuste dinâmico de taxa de quadros (60Hz / 120Hz / 144Hz) e throttling de abas em segundo plano para 1 frame/minuto.
- [ ] Metodologia formal de benchmarking automatizado comparativo contra Chromium e Firefox.

---

## ⚪ FASE 4 — Distribuição e Empacotamento
- [ ] Construção do pacote `.deb` para instalação nativa no Linux Mint e Ubuntu.
- [ ] Configuração de repositório APT assinado com GPG para atualizações automáticas via `apt upgrade`.
- [ ] Validação de builds reprodutíveis e geração automática de Software Bill of Materials (SBOM).
- [ ] Pipeline de CI/CD no GitHub Actions com auditoria de vulnerabilidades (`cargo audit`, `cargo deny`).

---

## ⚪ FASE 5 — Versão Beta Pública
- [ ] Testes de compatibilidade em larga escala com os 500 sites mais acessados.
- [ ] Auditoria externa de segurança e revisão completa de licenças de terceiros.
- [ ] Abertura do portal de suporte e sistema público de reporte de bugs.

---

## ⚪ FASE 6 — Expansão de Plataformas
- [ ] Arquitetura de abstração para backend gráfico no Windows (WinUI/WebView2/CEF).
- [ ] Suporte à arquitetura Linux ARM64 (Raspberry Pi 5, servidores ARM).
- [ ] Avaliação de backend nativo para macOS.
