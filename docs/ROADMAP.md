# Roteiro de Desenvolvimento (Roadmap) — BEAR Browser

O desenvolvimento do BEAR Browser é conduzido em etapas progressivas, guiadas por critérios objetivos de aceitação e comprovação de métricas.

---

## 🟢 FASE 0 — Auditoria, Arquitetura e Prova de Conceito (CONCLUÍDA)
- [x] Inspecionar o ambiente de desenvolvimento (Linux Mint 22 / Ubuntu 24.04 x86-64).
- [x] Elaborar o relatório técnico comparativo e o ADR-0001 (CEF vs Content API vs GTK4).
- [x] Estruturar o repositório modular e o workspace Cargo (`apps/bear-linux` + 8 crates).
- [x] Implementar a janela nativa GTK4/libadwaita com controles de navegação e abas.
- [x] Implementar navegação real na web (carregamento ao vivo de `https://kernel.org`).
- [x] Integrar motor de internacionalização com paridade de catálogos pt-BR e en.
- [x] Integrar persistência SQLite ACID para histórico e favoritos com migrações versionadas.
- [x] Implementar o Escudo de Privacidade com bloqueador de anúncios e interceptação de rede.
- [x] Validar sandbox seccomp e namespaces no kernel Linux.
- [x] Atingir benchmark de memória RSS inicial (<25 MB) e tempo de prontidão (<150 ms).

---

## 🟡 FASE 1 — Navegador Funcional e Experiência Diária (EM ANDAMENTO)
- [ ] Gerenciador de downloads com suporte a pausas, cancelamentos e retomadas.
- [ ] Renderizador completo de páginas HTML/CSS complexas via pipeline acelerado CEF OSR em GtkGLArea.
- [ ] Suporte a histórico expansível com pesquisa textual rápida e agrupamento por data.
- [ ] Barra de favoritos na interface com suporte a pastas e importação de outros navegadores.
- [ ] Integração com atalhos de zoom (`Ctrl++`, `Ctrl+-`, `Ctrl+0`) e busca na página (`Ctrl+F`).

---

## ⚪ FASE 2 — Privacidade Avançada e Bloqueio Granular
- [ ] Suporte completo a parsing de regras de sintaxe Adblock Plus / uBlock Origin via listas dinâmicas.
- [ ] Atualização segura de listas de filtros em segundo plano com verificação de assinatura digital.
- [ ] Painel detalhado de bloqueios por aba (lista de scripts e elementos bloqueados por domínio).
- [ ] Particionamento estrito de cookies de terceiros e proteção ativa contra canvas fingerprinting.
- [ ] Suporte a DNS over HTTPS (DoH) configurável com provedores focados em privacidade (Quad9, Mullvad).

---

## ⚪ FASE 3 — Desempenho e Otimizações de Memória
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
