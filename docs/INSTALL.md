# Guia de Instalação Oficial — Brunya Browser

O Brunya Browser está disponível para distribuições Linux baseadas em Debian e Ubuntu (com suporte oficial a **Linux Mint 22+** e **Ubuntu 24.04 LTS x86-64**).

---

## Método 1: Instalação via Repositório APT Oficial (Recomendado)

A instalação através do repositório APT assinado garante que você receba atualizações de segurança e melhorias automaticamente junto com as atualizações regulares do seu sistema operacional (`apt upgrade`).

### 1. Baixar e instalar a chave pública GPG oficial
```bash
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.brunyabrowser.org/keys/brunya-archive-keyring.gpg | sudo tee /etc/apt/keyrings/brunya-browser.gpg > /dev/null
```

### 2. Adicionar o repositório do Brunya Browser às fontes do APT
```bash
echo "deb [signed-by=/etc/apt/keyrings/brunya-browser.gpg] https://repo.brunyabrowser.org/ stable main" | sudo tee /etc/apt/sources.list.d/brunya-browser.list
```

### 3. Atualizar índices e instalar o navegador
```bash
sudo apt update
sudo apt install -y brunya-browser
```

### 4. Executar o Brunya Browser
- Pelo menu de aplicativos do seu ambiente gráfico (Cinnamon, GNOME, XFCE).
- Ou pelo terminal:
  ```bash
  brunya-browser
  ```

---

## Método 2: Instalação Manual via Pacote `.deb`

Se você preferir baixar diretamente o pacote instalável:

### 1. Baixar o pacote `.deb` oficial
Baixe a versão mais recente em [https://github.com/Tsujiu/brunya-browser/releases](https://github.com/Tsujiu/brunya-browser/releases) ou compile localmente:
```bash
./scripts/build-deb.sh
```

### 2. Instalar o pacote
Utilize o `apt` para instalar o arquivo `.deb`, pois ele resolve e instala automaticamente quaisquer dependências ausentes do sistema:
```bash
sudo apt install -y ./dist/brunya-browser_*_amd64.deb
```
*(Alternativamente: `sudo dpkg -i dist/brunya-browser_*_amd64.deb`)*

---

## 🗑️ Como Desinstalar

Para remover o Brunya Browser preservando suas preferências locais:
```bash
sudo apt remove brunya-browser
```

Para remover completamente o navegador, arquivos de configuração do sistema e alternativas:
```bash
sudo apt purge brunya-browser
```

Para remover também os dados locais da sua pasta pessoal (histórico, favoritos locais e cache):
```bash
rm -rf ~/.local/share/brunya ~/.config/brunya
```
