<p align="center">
  <img src=".github/assets/banner.png" alt="Mac Cleaner" width="100%">
</p>

<p align="center">
  <a href="../../actions/workflows/build.yml"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/Skuth/mac-cleaner/build.yml?branch=master&label=build&style=flat-square"></a>
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white">
  <img alt="React 19" src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?style=flat-square&logo=rust&logoColor=white">
  <img alt="macOS" src="https://img.shields.io/badge/macOS-12%2B-000000?style=flat-square&logo=apple&logoColor=white">
</p>

---

Um Mac de desenvolvimento junta lixo rápido. Runtime de simulador que você
instalou pra testar uma versão de iOS há dois anos. `DerivedData` de todo
projeto que já abriu no Xcode. Um emulador Android de 12 GB. Quarenta pastas
`node_modules` de projetos que nunca mais tocou. Imagens Docker de um
`docker-compose` que já foi deletado.

Nada disso aparece em "Armazenamento" nas Configurações do Sistema — o macOS
empilha tudo em "Outros" e te deseja boa sorte.

**Mac Cleaner acha essas coisas, mede cada uma, e te diz o que acontece se
você apagar.** Aí você decide.

## O que ele resolve

O problema de limpar disco não é *achar* o que ocupa espaço — `du -sh *` faz
isso. O problema é saber **o que dói perder**.

Apagar `~/Library/Developer/Xcode/DerivedData` é grátis: volta no próximo
build. Apagar `~/Library/Developer/Xcode/Archives` te custa os dSYMs de
simbolicação de crash de versões que já estão na App Store — e não volta
nunca.

Os dois são pastas do Xcode com nome parecido, no mesmo diretório. Um
limpador genérico trata igual. Aqui não.

Cada item da lista mostra três coisas:

| | |
|---|---|
| **O que é** | Uma frase em português, sem jargão. |
| **Quanto ocupa** | Medido de verdade, com `du`, na hora do scan. |
| **Como volta** | O comando, o download ou o "não volta". |

E é agrupado por risco:

| Risco | Significa | Exemplo |
|---|---|---|
| 🟢 **Regenera** | Volta sozinho, sem você fazer nada. | `DerivedData`, cache do Vite, miniaturas do Quick Look |
| 🟡 **Rebaixa** | Volta com um comando ou um download. | Cache do Gradle, store do pnpm, runtime de simulador |
| 🔴 **Some** | Não volta, ou custa caro recuperar. | Archives do Xcode, volumes Docker, mídia do WhatsApp |

O botão **Marcar recomendados** liga só o que regenera sozinho — e ainda tira
dessa seleção o que tecnicamente regenera mas custa dez minutos e uma conexão
(navegadores do Playwright, binários do Electron, Cypress). Artefato por
projeto (`node_modules`, `target/`) também fica de fora de propósito: só você
sabe qual projeto está dormindo.

## O que ele encontra

<details open>
<summary><b>Simuladores iOS</b></summary>

- Cada runtime instalado, um item por versão de iOS/watchOS/tvOS — costuma ser
  o maior achado da lista (7 GB cada)
- Dados dos simuladores (apps instalados, fotos, bancos de dados)
- Cache do CoreSimulator

A limpeza roda `simctl` quando existe comando oficial, porque ele desregistra
o que apaga — `rm -rf` deixaria o Xcode achando que o runtime ainda está lá.
</details>

<details>
<summary><b>Xcode</b></summary>

- `DerivedData` — índices, módulos e binários intermediários de todo projeto
- `iOS DeviceSupport` — símbolos de debug de cada aparelho físico já conectado,
  uma cópia por versão de iOS
- `Archives` — 🔴 builds de distribuição com os dSYMs de simbolicação
- Cache do SwiftUI Previews
- `Products`
- Cache do app Xcode
</details>

<details>
<summary><b>Android</b></summary>

- Cada AVD (emulador), um item por device
- NDKs instalados, um item por versão
- Imagens de sistema
- Cache do Gradle, logs do daemon, distribuições do Gradle Wrapper
- Caches do Android Studio
- Cache do emulador
</details>

<details>
<summary><b>Node & JavaScript</b></summary>

- `node_modules` por projeto, com o nome do projeto na frente
- Caches de gerenciador: npm, pnpm, Yarn, Bun, Deno
- Headers do node-gyp, cache do Vite
- Binários de teste: Cypress, Playwright, Puppeteer, Electron
- Versões de Node instaladas via nvm
</details>

<details>
<summary><b>Docker</b></summary>

- Imagens e contêineres parados
- Cache de build
- 🔴 Volumes sem contêiner (é onde mora banco de dados de dev)
- 🔴 Disco virtual do Docker — reset total

Usa `docker system df` pra medir e as subcomandos oficiais pra limpar.
</details>

<details>
<summary><b>Builds & toolchains</b></summary>

- `target/` por projeto Rust
- Registry do Cargo, 🔴 toolchains do rustup
- Módulos do Go, gems do Ruby
- Cache do pip e do uv
- Cache do Composer, pacotes NuGet
- Downloads do Homebrew
</details>

<details>
<summary><b>Sistema & apps</b></summary>

- Lixeira
- Papéis de parede em vídeo, assets baixados do sistema
- Logs e relatórios de crash
- Miniaturas do Quick Look
- Cache do VS Code (rede e bytecode V8), Slack, Spotify, Discord
- Cache do Chrome e do Edge — só cache, não toca em senha, histórico nem aba
- 🔴 Mídia do WhatsApp
- Caches restantes de apps
</details>

Alvo abaixo de 1 MB nem aparece — só polui a tela. Projetos são procurados em
`~/Documents/code`, `~/Documents/projects`, `~/Developer`, `~/code`, `~/dev`,
`~/projects` e `~/workspace`.

## Segurança

Um app que apaga arquivos com `rm -rf` merece desconfiança. Como esse aqui se
protege:

**O front-end nunca envia caminho.** Ele manda o `id` de um alvo que o scan
devolveu. A ação destrutiva correspondente fica guardada num `Mutex<HashMap>`
no Rust, montada pelo backend. Uma tela comprometida não consegue pedir a
remoção de um caminho arbitrário — não existe API pra isso.

**`clean::guard` roda antes de cada remoção**, e recusa:

- caminho relativo
- symlink
- qualquer coisa fora de `$HOME` ou `/Library/Developer`
- caminho com menos de 4 componentes (`/Users`, `/Users/você`, `/Library`)
- as pastas de conteúdo do usuário: `Documents`, `Desktop`, `Downloads`,
  `Pictures`, `Movies`, `Music`, `Library`, `Applications`

Isso não é defesa contra o front — é a rede contra um bug no catálogo. Um
`join()` com string vazia vira a home inteira sem reclamar de nada.

**Sem `sudo`, sem privilégio elevado.** O app roda como você. O que você não
pode apagar no Finder, ele também não pode.

**Nada vai pra Lixeira.** O que é apagado, é apagado — por isso a confirmação
lista item por item, com tamanho, e destaca em vermelho o que não volta.

## Rodando

Precisa de [Node 20+](https://nodejs.org), [Rust
estável](https://rustup.rs) e Xcode Command Line Tools.

```sh
npm install
npm run tauri dev      # desenvolvimento, com hot reload no front
npm run tauri build    # gera o .app e o .dmg
```

O build sai em `src-tauri/target/release/bundle/`.

### Testes

```sh
cd src-tauri && cargo test
```

Cobrem o parser de tamanho do `docker system df`, a ausência de ids duplicados
no catálogo, a regra de quem entra em "recomendados" e — o que importa — que o
`guard` recusa os caminhos que não podem sumir.

## Build no CI

O workflow é **manual**, não roda em push. Em **Actions → Build → Run
workflow**, na branch `master`.

Compila as três plataformas em paralelo, roda `cargo test` antes de empacotar
e sobe os instaladores como artifacts:

| Plataforma | Runner | Saída |
|---|---|---|
| macOS (universal) | `macos-latest` | `.dmg`, `.app` |
| Linux x86_64 | `ubuntu-22.04` | `.deb`, `.rpm`, `.AppImage` |
| Windows x86_64 | `windows-latest` | `.msi`, `.exe` |

> **Nota.** O código compila nas três, mas o catálogo é macOS: mede com `du`,
> lê `df /System/Volumes/Data` e procura em `~/Library`. Os binários de Linux e
> Windows abrem e não acham nada pra limpar. Estão lá porque a pipeline pede —
> pra virarem úteis, `catalog.rs` precisa de um bloco `#[cfg(target_os)]` com
> os caminhos de cada sistema.

## Como está montado

Tauri 2 (Rust) + React 19 + Tailwind 4 + shadcn/ui, estado em Zustand.

```
src-tauri/src/
  catalog.rs   o que pode ser limpo — só descreve, não apaga
  scan.rs      medição via `du -sk`, em paralelo
  clean.rs     a única parte que escreve no disco
  types.rs     Action, Spec, Target, Risk
  lib.rs       registra os comandos e guarda as ações do último scan
src/
  store.ts     estado (zustand)
  App.tsx      layout, confirmação, progresso
  components/  DiskHeader, TargetList, CleaningOverlay
```

O scan roda `du` em paralelo (2 a 8 threads, conforme a CPU) e emite
`scan:progress` a cada alvo medido — sem isso a primeira tela ficaria minutos
parada. A limpeza emite `clean:progress` do mesmo jeito.

## Adicionando um alvo novo

Uma entrada em `catalog.rs`, dentro do grupo que fizer sentido:

```rust
specs.push(Spec {
    id: "grupo.item".into(),
    group: G_NODE,
    label: "Nome curto".into(),
    detail: "O que é isso, em uma frase.".into(),
    recovery: "Como volta.".into(),
    risk: Risk::Safe,
    size: SizeOf::Du(vec![p.clone()]),
    action: Action::empty(vec![p]),
});
```

As três etapas de `Action` rodam nessa ordem:

| | |
|---|---|
| `Action::exec` | Roda a ferramenta oficial (`simctl`, `docker`) — ela sabe desregistrar o que apaga. Falha aqui não impede as etapas seguintes. |
| `Action::remove` | Leva o caminho inteiro. |
| `Action::empty` | Esvazia o diretório mas mantém ele existindo — alguns apps quebram se o próprio diretório sumir. |

Podem ser combinadas no mesmo `Action`. Só use `live(&p)` antes de empurrar o
spec: alvo que não existe não deve entrar na lista.

Escrever o `detail` e o `recovery` com cuidado é metade do trabalho — é o que
diferencia esse app de um `rm -rf` com interface.

## Licença

MIT.
