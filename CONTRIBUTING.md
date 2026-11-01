# Guia de Contribuição — BEAR Browser

Agradecemos o seu interesse em contribuir com o BEAR Browser! Este é um projeto de código aberto focado em construir um navegador web rápido, leve e privado.

---

## 1. Princípios de Engenharia

Antes de submeter código, certifique-se de seguir as diretrizes do projeto:
1. **Sem Funcionalidades Fictícias:** Toda funcionalidade implementada deve possuir comportamento real, tratamento de erros e testes.
2. **Segurança Primeiro:** Desempenho nunca deve comprometer proteções fundamentais de segurança ou sandbox.
3. **Internacionalização:** Nenhum texto da interface pode ser fixo (hardcoded) no código Rust. Use sempre o sistema de tradução `bear-i18n` com chaves em pt-BR e en.
4. **Respeito à Memória:** Evite trabalho redundante na thread principal da interface. Use tipos eficientes e concorrência limpa.

---

## 2. Processo de Desenvolvimento

1. Crie uma branch para a sua funcionalidade ou correção:
   ```bash
   git checkout -b feature/minha-melhoria
   ```
2. Escreva testes automatizados cobrindo a alteração.
3. Verifique se o código compila e passa em todos os testes e formatações:
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. Submeta seu Pull Request com uma descrição clara da motivação técnica e resultados observados.
