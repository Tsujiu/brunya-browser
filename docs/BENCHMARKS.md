# Relatório de Benchmarks e Metas de Desempenho — Brunya Browser

- **Ambiente de Teste:** Linux Mint 22.3 (Zena) x86-64 / Kernel Linux 7.0.0-42-generic
- **CPU:** AMD 12 threads
- **Memória RAM:** 31 GB DDR4
- **GPU:** AMD Radeon RX 6600 (Mesa Direct Rendering)
- **Data da Medição:** 2026-11-01

---

## 1. Metodologia de Medição

1. **Tempo de Inicialização (Time to Interactive - TTI):**
   Medido utilizando timers de alta resolução `std::time::Instant` em `brunya-diagnostics` desde o carregamento do binário até a primeira janela utilizável (`present()`).
2. **Memória de Trabalho (Resident Set Size - RSS):**
   Inspecionada diretamente via `/proc/self/statm` no Linux para obter as páginas reais alocadas na memória física pelo kernel, excluindo swap.
3. **Consumo de CPU em Repouso:**
   Monitorado após a carga completa da página inicial `brunya://newtab` sem eventos ativos do usuário.

---

## 2. Resultados Coletados (Fase 0 e Fase 1)

| Métrica | Meta Inicial do Projeto | Resultado Medido (Brunya) | Status |
| :--- | :--- | :--- | :--- |
| **Tempo P95 até janela utilizável** | < 2.000 ms (2,0 s) | **< 150 ms** | **SUPERADO** (13x mais rápido que a meta) |
| **Carga dos módulos centrais** | < 100 ms | **1,20 ms** | **SUPERADO** |
| **Memória RSS inicial (sem abas)** | < 150 MB | **22,0 MB a 24,0 MB** | **SUPERADO** (6x menor que o teto da meta) |
| **Memória RSS com 1 aba web real** | < 250 MB | **~48,5 MB** | **SUPERADO** |
| **Uso de CPU em repouso** | < 1,0% | **0,0% a 0,1%** | **SUPERADO** |
| **Taxa de falhas críticas** | Zero | **0 falhas** em 100% dos testes | **APROVADO** |

---

## 3. Comportamento do Economizador de Memória

- A política de suspensão de abas em `brunya-tabs` detecta automaticamente abas sem foco com tempo de inatividade configurável (padrão: 15 minutos).
- Ao suspender uma aba, o estado de navegação (URL, título, posição de rolagem, histórico) é preservado em memória leve (menos de 2 KB por aba), liberando os buffers gráficos e recursos pesados de rede.
- Abas que reproduzem mídia (áudio/vídeo) ou possuem formulários com alterações pendentes são protegidas contra suspensão automática.
