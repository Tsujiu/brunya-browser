# Arquitetura Técnica — Brunya Browser

O Brunya Browser foi projetado sob os princípios de modularidade estrita, desacoplamento do motor de renderização, gerenciamento previsível de memória e segurança em camadas.

---

## 1. Visão Geral dos Componentes

```text
+----------------------------------------------------------------------------------+
|                                   BRUNYA-APP                                     |
|                  (Interface GTK4 + libadwaita: Janelas, Abas, Menus)             |
+----------------------------------------------------------------------------------+
        |                                     |                              |
        v                                     v                              v
+------------------+                +------------------+           +------------------+
|   BRUNYA-TABS    |                | BRUNYA-SETTINGS  |           |   BRUNYA-I18N    |
| (Gestão de Abas, |                | (Preferências,   |           | (pt-BR / en,     |
|   Suspensão)     |                |     Temas)       |           |   Interpolação)  |
+------------------+                +------------------+           +------------------+
        |                                     |                              |
        +------------------+------------------+------------------------------+
                           |
                           v
+----------------------------------------------------------------------------------+
|                                   BRUNYA-CORE                                    |
|             (Traits BrowserEngine / BrowserTab, TabId, NavigationState)          |
+----------------------------------------------------------------------------------+
        |                                     |                              |
        v                                     v                              v
+------------------+                +------------------+           +------------------+
|  BRUNYA-ADBLOCK  |                |  BRUNYA-STORAGE  |           |  BRUNYA-PRIVACY  |
| (Interceptação   |                | (SQLite ACID,    |           | (Permissões de   |
|   de Rede)       |                |   Migrações)     |           |  Site, Cookies)  |
+------------------+                +------------------+           +------------------+
        |
        v
+----------------------------------------------------------------------------------+
|                              MOTOR DE NAVEGAÇÃO                                  |
|                     (Chromium 156 / CEF OSR Pipeline)                            |
+----------------------------------------------------------------------------------+
```

---

## 2. Decisões Arquiteturais Fundamentais

### 2.1. Desacoplamento do Motor Web (`brunya-core`)
Nenhum módulo da interface gráfica ou das regras de negócio depende diretamente de tipos concretos de um motor web específico. O acesso ao motor ocorre exclusivamente através das interfaces `BrowserEngine`, `BrowserTab` e `NetworkInterceptor`.
Isso permite:
- Testabilidade unitária e de integração sem a necessidade de instanciar o Chromium completo.
- Atualizações transparentes do CEF ou substituição controlada de subprocessos sem quebras na interface do usuário.

### 2.2. Renderização Off-Screen (OSR) para GTK4
No GTK4, o modelo clássico de janelas X11 estrangeiras (`XReparentWindow`) não é suportado pelo Scene Kit (GSK) e é incompatível com o protocolo Wayland.
O Brunya utiliza Off-Screen Rendering (OSR):
- O motor renderiza quadros em um buffer de memória compartilhada / textura GPU.
- A interface GTK4 consome esses quadros via widgets nativos (`GdkMemoryTexture` / `GtkGLArea`), viabilizando:
  - 100% de compatibilidade tanto sob Wayland quanto sob X11.
  - Cantos arredondados nativos do libadwaita, sombras, popovers e transições fluidas sobre a página web.

### 2.3. Interceptação de Rede em Nível de Protocolo (`brunya-adblock`)
Diferente de navegadores que dependem de extensões WebExtension em JavaScript (que só agem após o DOM ser injetado), o motor do Brunya intercepta requisições HTTP/HTTPS no estágio de requisição do motor (`NetworkInterceptor`), cancelando o download antes que os dados trafeguem pela rede, economizando banda e tempo de CPU.

### 2.4. Persistência e Isolamento de Dados (`brunya-storage` & `brunya-privacy`)
- Todos os dados do usuário (histórico, favoritos, configurações) residem localmente em bancos SQLite versionados com transações ACID.
- No **Modo de Navegação Privada**, nenhuma permissão ou dado de navegação é gravado no disco; as estruturas de estado são alocadas em memória volátil e limpas imediatamente ao fechar a janela privada.
