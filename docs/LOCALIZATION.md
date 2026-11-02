# Diretrizes de Internacionalização e Tradução — Brunya Browser

O Brunya Browser foi concebido desde a sua primeira linha de código como um software global, com suporte a múltiplos idiomas sem acoplamento da lógica de negócios aos textos visíveis.

---

## 1. Idiomas Oficiais

1. **Português brasileiro (`pt-BR`):** Idioma de referência e padrão primário do projeto.
2. **Inglês (`en`):** Segundo idioma oficial obrigatório.

---

## 2. Estrutura dos Catálogos

Os catálogos residem em `locales/<idioma>/` e estão organizados por domínio funcional:

```text
locales/
├── pt-BR/
│   ├── common.json        # Ações globais, menus e botões da interface
│   ├── settings.json      # Rótulos da tela de configurações
│   ├── privacy.json       # Termos de segurança, escudos e permissões
│   ├── downloads.json     # Termos do gerenciador de downloads
│   └── errors.json        # Mensagens de erro de rede e conexão
└── en/
    ├── common.json
    ├── settings.json
    ├── privacy.json
    ├── downloads.json
    └── errors.json
```

---

## 3. Regras para Contribuição de Novas Traduções

1. **Chaves Estáveis:** Todas as chaves devem ser nomeadas em `snake_case` e agrupadas pelo nome do arquivo (exemplo: `common.new_tab`, `privacy.shield_title`).
2. **Paridade Obrigatória:** Um idioma só pode ser adicionado se implementar 100% das chaves existentes em `pt-BR`. O teste automatizado `brunya-i18n::tests::test_catalogues_parity` valida a equivalência estrita das chaves em tempo de compilação.
3. **Interpolação de Variáveis:** Parâmetros dinâmicos utilizam a sintaxe `{nome_da_variavel}` (exemplo: `{count} rastreadores bloqueados`).
4. **Fallback:** Caso uma chave não seja encontrada no idioma selecionado pelo usuário, o sistema aplica automaticamente o fallback para `pt-BR`.
