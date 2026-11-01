# Política de Segurança — BEAR Browser

A segurança e a privacidade dos usuários são princípios inegociáveis no desenvolvimento do BEAR Browser.

---

## 1. Modelo de Ameaças e Sandbox

O BEAR Browser adota o modelo de segurança multi-nível do ecossistema Chromium no Linux:
1. **User Namespaces / SUID Sandbox:** Isolamento estrito de processos de renderização não confiáveis.
2. **Filtros Seccomp-bpf:** Bloqueio de chamadas de sistema (syscalls) desnecessárias nos processos de renderização e utilitários.
3. **Isolamento de Sites (Site Isolation):** Processos de renderização distintos para origens diferentes, mitigando ataques de canais laterais (Spectre / Meltdown).
4. **Privilégios Mínimos:** A interface principal do navegador nunca é executada como superusuário (root).

---

## 2. Reporte de Vulnerabilidades

Caso você encontre uma vulnerabilidade de segurança no BEAR Browser, por favor **NÃO** abra uma issue pública.

Envie um relatório detalhado para:
`security@bearbrowser.org`

Inclua no relatório:
- Versão exata do BEAR Browser e distribuição Linux utilizada.
- Passos para reprodução (Proof of Concept - PoC).
- Impacto estimado da vulnerabilidade.

Nossa equipe se compromete a:
- Confirmar o recebimento em até 48 horas.
- Fornecer uma avaliação técnica e prazo estimado de correção.
- Coordenar a divulgação responsável após a disponibilização do patch.
