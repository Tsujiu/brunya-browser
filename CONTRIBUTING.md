# Como contribuir — Brunya Browser

O código é público para consulta. Não é código aberto. A reutilização
está limitada pela minuta em [LICENSE](LICENSE). Enviar um Pull Request
não transfere a titularidade de quem contribui.

Leia [CLA.md](CLA.md) antes de abrir a proposta. A versão vigente é
`CLA-BRUNYA-2026-11-02`.

## O que pode ser proposto

Correções, testes e melhorias ao próprio Brunya Browser, para serem
analisadas pelo mantenedor. O mantenedor pode recusar qualquer proposta,
sem dever de integrar.

Um fork só é admitido pela licença para preparar essa proposta e
enquanto ela estiver aberta. Fork como produto separado não está
autorizado.

## Antes de abrir o Pull Request

1. Confirme que você pode licenciar o que está enviando.
2. Não inclua segredo, `.env`, certificado, banco de dados, dado pessoal
   ou material de terceiro sem indicar a licença no texto da proposta.
3. Crie uma branch a partir de `main`:

   ```bash
   git checkout -b correcao/descricao-curta
   ```

4. Escreva um teste quando a mudança tiver comportamento verificável.
5. Rode, na raiz do repositório:

   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

6. No corpo do Pull Request, copie esta frase sem alterá-la:

   ```text
   Aceite do CLA: eu li CLA.md (CLA-BRUNYA-2026-11-02) e concedo ao titular a licença não exclusiva descrita nesse documento.
   ```

Sem essa frase, o fluxo de verificação não considera o CLA aceito. O
arquivo CLA.md, sozinho, não registra aceite.

## O que o mantenedor espera no código

- A função precisa ter comportamento real e tratamento de erro. Não
  envie caminho que só simula o resultado.
- Texto de interface entra em `locales/pt-BR` e `locales/en`, pelas
  chaves de `brunya-i18n`.
- Trabalho pesado não deve travar a thread da interface GTK.
- Não altere `LICENSE` nem `CLA.md` dentro de uma contribuição comum. Uma
  mudança desses arquivos pede conversa prévia com o mantenedor.

## Revisão

`main` deve receber mudança por Pull Request, com aprovação de quem está
em [.github/CODEOWNERS](.github/CODEOWNERS). Conversa de revisão aberta
segura o merge. Force push e apagar `main` ficam bloqueados quando essa
proteção está ativa no GitHub. A proteção é da plataforma. Ela não está
válida só porque este arquivo existe. O estado real aparece nas
configurações do repositório.

## Conduta

O texto em [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) vale para issues e
Pull Requests. Vulnerabilidade não vai em issue pública. O caminho está
em [SECURITY.md](SECURITY.md).
