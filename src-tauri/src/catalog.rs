//! Monta a lista de alvos de limpeza. Nada aqui apaga coisa alguma — só
//! descreve o que *poderia* ser apagado, com o custo de recuperar cada item.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::{Action, Risk, SizeOf, Spec};

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/Users/nobody".into()))
}

/// Acesso Total ao Disco. Sem ele, tocar em ~/Documents ou no contêiner de
/// outro app dispara prompt do TCC — e com `du` em paralelo, dispara em
/// rajada. O TCC.db só abre com o acesso, e falhar aqui não gera prompt.
pub fn full_disk_access() -> bool {
    fs::File::open(home().join("Library/Application Support/com.apple.TCC/TCC.db")).is_ok()
}

/// Só interessa alvo que existe e não está vazio.
fn live(p: &Path) -> bool {
    match fs::read_dir(p) {
        Ok(mut entries) => entries.next().is_some(),
        // Arquivo solto (não-diretório) também conta.
        Err(_) => p.is_file(),
    }
}

/// Filhos diretos de um diretório, ordenados pra saída ficar estável.
fn children(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

fn name_of(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().to_string()
}

/// Onde procurar projetos. Só os que existem de verdade.
fn dev_roots() -> Vec<PathBuf> {
    let h = home();
    let fda = full_disk_access();
    [
        "Documents/code",
        "Documents/projects",
        "Developer",
        "code",
        "dev",
        "projects",
        "workspace",
    ]
    .iter()
    // Sem acesso, nem encosta em ~/Documents: o `is_dir` já pediria permissão.
    .filter(|r| fda || !r.starts_with("Documents/"))
    .map(|r| h.join(r))
    .filter(|p| p.is_dir())
    .collect()
}

/// Acha diretórios com um nome específico, sem descer dentro do que achou.
/// `depth` é quantos níveis abaixo da raiz, contando o próprio alvo.
fn find_dirs(roots: &[PathBuf], name: &str, depth: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, name: &str, left: usize, out: &mut Vec<PathBuf>) {
        if left == 0 {
            return;
        }
        for child in children(dir) {
            if !child.is_dir() || child.is_symlink() {
                continue;
            }
            let base = name_of(&child);
            if base == name {
                out.push(child);
                continue; // não desce dentro do que já é alvo
            }
            // Diretório oculto raramente guarda projeto e explode o tempo de scan.
            if base.starts_with('.') {
                continue;
            }
            walk(&child, name, left - 1, out);
        }
    }

    let mut out = Vec::new();
    for root in roots {
        walk(root, name, depth, &mut out);
    }
    out
}

/// Nome legível de um projeto a partir do caminho do artefato dentro dele.
fn project_of(artifact: &Path) -> String {
    artifact
        .parent()
        .map(|p| name_of(p))
        .unwrap_or_else(|| "?".into())
}

fn run(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        None
    }
}

fn cmd(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Grupos
// ---------------------------------------------------------------------------

const G_IOS: &str = "Simuladores iOS";
const G_XCODE: &str = "Xcode";
const G_ANDROID: &str = "Android";
const G_NODE: &str = "Node & JavaScript";
const G_DOCKER: &str = "Docker";
const G_BUILD: &str = "Builds & toolchains";
const G_SYS: &str = "Sistema & apps";

/// O que o botão "Marcar recomendados" liga.
///
/// A base é tudo que regenera sozinho. As exceções existem porque "regenera
/// sozinho" às vezes custa dez minutos e uma conexão: um cache de 500 MB de
/// navegador de teste tecnicamente volta sem você fazer nada, mas volta *na
/// hora errada*. E artefato por projeto fica de fora de propósito — só você
/// sabe qual projeto está dormindo.
pub fn is_recommended(id: &str, risk: Risk) -> bool {
    // A Lixeira é o único caso fora de Safe que entra: você já jogou fora.
    if id == "sys.trash" {
        return true;
    }
    if risk != Risk::Safe {
        return false;
    }
    const EXCECOES: [&str; 6] = [
        "node.modules.",   // qual projeto está dormindo é escolha sua
        "build.target.",   // idem, e recompilar Rust dói
        "node.playwright", // ~1 GB de navegadores por download
        "node.cypress",
        "node.puppeteer",
        "node.electron",
    ];
    !EXCECOES.iter().any(|e| id.starts_with(e))
}

pub fn build() -> Vec<Spec> {
    let mut specs = Vec::new();
    ios(&mut specs);
    xcode(&mut specs);
    android(&mut specs);
    node(&mut specs);
    docker(&mut specs);
    build_artifacts(&mut specs);
    system(&mut specs);
    specs
}

// --- iOS -------------------------------------------------------------------

fn ios(specs: &mut Vec<Spec>) {
    // Um alvo por runtime, com o tamanho que o próprio simctl informa.
    if let Some(json) = run("xcrun", &["simctl", "runtime", "list", "-j"]) {
        if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&json) {
            for (uuid, rt) in map {
                let version = rt["version"].as_str().unwrap_or("?").to_string();
                let build = rt["build"].as_str().unwrap_or("?").to_string();
                let platform = rt["platformIdentifier"]
                    .as_str()
                    .unwrap_or("com.apple.platform.iphonesimulator")
                    .rsplit('.')
                    .next()
                    .unwrap_or("iphonesimulator")
                    .to_string();
                let os = if platform.contains("watch") {
                    "watchOS"
                } else if platform.contains("tv") {
                    "tvOS"
                } else if platform.contains("xr") {
                    "visionOS"
                } else {
                    "iOS"
                };
                let bytes = rt["sizeBytes"].as_u64().unwrap_or(0);
                if bytes == 0 {
                    continue;
                }
                specs.push(Spec {
                    id: format!("ios.runtime.{uuid}"),
                    group: G_IOS,
                    label: format!("Runtime {os} {version} ({build})"),
                    detail: "Imagem de disco completa do sistema, usada pelos simuladores dessa \
                             versão. Cada runtime é independente — apagar um não afeta os outros."
                        .into(),
                    recovery: "Xcode › Settings › Components rebaixa em ~15-25 min.".into(),
                    risk: Risk::Low,
                    size: SizeOf::Fixed(bytes),
                    action: Action::exec(vec![cmd(&[
                        "xcrun", "simctl", "runtime", "delete", &uuid,
                    ])]),
                });
            }
        }
    }

    let devices = home().join("Library/Developer/CoreSimulator/Devices");
    if live(&devices) {
        specs.push(Spec {
            id: "ios.devices".into(),
            group: G_IOS,
            label: "Dados dos simuladores".into(),
            detail: "Apps instalados, bancos, fotos e ajustes de cada simulador criado. Os \
                     aparelhos voltam a existir, só que zerados."
                .into(),
            recovery: "Xcode recria na hora. Você reinstala o app no simulador.".into(),
            risk: Risk::Low,
            size: SizeOf::Du(vec![devices.clone()]),
            action: Action {
                exec: vec![
                    cmd(&["xcrun", "simctl", "shutdown", "all"]),
                    cmd(&["xcrun", "simctl", "delete", "all"]),
                ],
                empty: vec![devices],
                ..Default::default()
            },
        });
    }

    let caches = home().join("Library/Developer/CoreSimulator/Caches");
    if live(&caches) {
        specs.push(Spec {
            id: "ios.caches".into(),
            group: G_IOS,
            label: "Cache do CoreSimulator".into(),
            detail: "Cache de dyld e arquivos temporários do serviço de simulação.".into(),
            recovery: "Regenera sozinho. Só o primeiro boot do simulador fica lento.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![caches.clone()]),
            action: Action::empty(vec![caches]),
        });
    }
}

// --- Xcode -----------------------------------------------------------------

fn xcode(specs: &mut Vec<Spec>) {
    let dev = home().join("Library/Developer");

    let items: [(&str, &str, &str, &str, Risk); 5] = [
        (
            "xcode.derived",
            "DerivedData",
            "Índices, módulos e binários intermediários de todo projeto que você já abriu no Xcode.",
            "Regenera no próximo build. Costuma até resolver build quebrado.",
            Risk::Safe,
        ),
        (
            "xcode.devicesupport",
            "iOS DeviceSupport",
            "Símbolos de debug copiados de cada iPhone/iPad físico que você já conectou, uma cópia por versão de iOS.",
            "Recopia do aparelho no próximo debug (alguns minutos).",
            Risk::Safe,
        ),
        (
            "xcode.archives",
            "Archives",
            "Builds arquivados de distribuição, com os dSYMs de simbolicação de crash.",
            "Não volta. Só apague se as versões já subiram pra App Store.",
            Risk::Medium,
        ),
        (
            "xcode.previews",
            "Cache do SwiftUI Previews",
            "Simuladores e binários que o canvas de preview do Xcode mantém em paralelo.",
            "Regenera ao abrir um preview.",
            Risk::Safe,
        ),
        (
            "xcode.products",
            "Products",
            "Binários compilados exportados pelo Xcode.",
            "Regenera no próximo build.",
            Risk::Safe,
        ),
    ];
    let paths = [
        "Xcode/DerivedData",
        "Xcode/iOS DeviceSupport",
        "Xcode/Archives",
        "Xcode/UserData/Previews",
        "Xcode/Products",
    ];

    for ((id, label, detail, recovery, risk), rel) in items.iter().zip(paths) {
        let p = dev.join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: (*id).into(),
            group: G_XCODE,
            label: (*label).into(),
            detail: (*detail).into(),
            recovery: (*recovery).into(),
            risk: *risk,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }

    let caches = home().join("Library/Caches/com.apple.dt.Xcode");
    if live(&caches) {
        specs.push(Spec {
            id: "xcode.caches".into(),
            group: G_XCODE,
            label: "Cache do app Xcode".into(),
            detail: "Cache interno do Xcode: downloads parciais, índices de documentação.".into(),
            recovery: "Regenera sozinho.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![caches.clone()]),
            action: Action::empty(vec![caches]),
        });
    }

    // CocoaPods e Carthage guardam cópia de todo pod que você já instalou.
    for (id, rel, label, detail) in [
        (
            "xcode.cocoapods",
            "Library/Caches/CocoaPods",
            "Cache do CocoaPods",
            "Cópia de cada versão de pod já baixada, compartilhada entre projetos.",
        ),
        (
            "xcode.carthage",
            "Library/Caches/org.carthage.CarthageKit",
            "Cache do Carthage",
            "Fontes e binários de dependências do Carthage.",
        ),
    ] {
        let p = home().join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: id.into(),
            group: G_XCODE,
            label: label.into(),
            detail: detail.into(),
            recovery: "Rebaixa no próximo `pod install` / `carthage bootstrap`.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }
}

// --- Android ---------------------------------------------------------------

fn android(specs: &mut Vec<Spec>) {
    let h = home();

    // Um alvo por emulador: cada AVD costuma passar de 8 GB sozinho.
    let avd_dir = h.join(".android/avd");
    for avd in children(&avd_dir) {
        if avd.extension().and_then(|e| e.to_str()) != Some("avd") {
            continue;
        }
        let name = name_of(&avd).trim_end_matches(".avd").to_string();
        let ini = avd_dir.join(format!("{name}.ini"));
        specs.push(Spec {
            id: format!("android.avd.{name}"),
            group: G_ANDROID,
            label: format!("Emulador {}", name.replace('_', " ")),
            detail: "Disco virtual do emulador, com o Android instalado, seus apps e os dados \
                     deles. É quase sempre o item mais gordo do Android."
                .into(),
            recovery: "Device Manager recria em ~2 min, mas zerado — perde os apps de dentro."
                .into(),
            risk: Risk::Low,
            size: SizeOf::Du(vec![avd.clone()]),
            action: Action::remove(vec![avd, ini]),
        });
    }

    let sdk = h.join("Library/Android/sdk");

    // Um alvo por versão de NDK.
    for ndk in children(&sdk.join("ndk")) {
        if !ndk.is_dir() {
            continue;
        }
        let v = name_of(&ndk);
        specs.push(Spec {
            id: format!("android.ndk.{v}"),
            group: G_ANDROID,
            label: format!("NDK {v}"),
            detail: "Toolchain de compilação nativa C/C++. Só faz falta se algum projeto seu \
                     compila código nativo — React Native e Flutter puros não usam."
                .into(),
            recovery: "SDK Manager rebaixa (~1 GB por versão).".into(),
            risk: Risk::Low,
            size: SizeOf::Du(vec![ndk.clone()]),
            action: Action::remove(vec![ndk]),
        });
    }

    let sys_images = sdk.join("system-images");
    if live(&sys_images) {
        specs.push(Spec {
            id: "android.sysimages".into(),
            group: G_ANDROID,
            label: "Imagens de sistema".into(),
            detail: "As imagens de Android que os emuladores usam como base. Apagar aqui só \
                     importa se você ainda for criar emuladores dessas APIs."
                .into(),
            recovery: "SDK Manager rebaixa sob demanda.".into(),
            risk: Risk::Low,
            size: SizeOf::Du(vec![sys_images.clone()]),
            action: Action::empty(vec![sys_images]),
        });
    }

    for (id, rel, label, detail, recovery, risk) in [
        (
            "android.gradle.caches",
            ".gradle/caches",
            "Cache do Gradle",
            "Todo jar, aar e módulo já baixado por qualquer build Gradle, somado a builds \
             incrementais antigos.",
            "Rebaixa no próximo build. Só esse build fica lento.",
            Risk::Safe,
        ),
        (
            "android.gradle.daemon",
            ".gradle/daemon",
            "Logs do daemon Gradle",
            "Logs de execução dos daemons de build.",
            "Regenera sozinho.",
            Risk::Safe,
        ),
        (
            "android.gradle.wrapper",
            ".gradle/wrapper",
            "Distribuições do Gradle Wrapper",
            "Uma cópia completa do Gradle por versão que algum projeto seu pediu.",
            "O wrapper rebaixa automático no próximo build.",
            Risk::Safe,
        ),
        (
            "android.studio.caches",
            "Library/Caches/Google",
            "Caches do Android Studio",
            "Índices e caches do IDE. Costuma acumular uma pasta por versão instalada.",
            "Regenera sozinho — a primeira indexação do projeto fica lenta.",
            Risk::Safe,
        ),
        (
            "android.emulator.cache",
            ".android/cache",
            "Cache do emulador",
            "Arquivos temporários do emulador do Android.",
            "Regenera sozinho.",
            Risk::Safe,
        ),
    ] {
        let p = h.join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: id.into(),
            group: G_ANDROID,
            label: label.into(),
            detail: detail.into(),
            recovery: recovery.into(),
            risk,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }
}

// --- Node ------------------------------------------------------------------

fn node(specs: &mut Vec<Spec>) {
    let h = home();
    let roots = dev_roots();

    // Um alvo por projeto — você escolhe quais projetos estão dormindo.
    for nm in find_dirs(&roots, "node_modules", 4) {
        specs.push(Spec {
            id: format!("node.modules.{}", nm.to_string_lossy()),
            group: G_NODE,
            label: format!("node_modules · {}", project_of(&nm)),
            detail: nm.to_string_lossy().to_string(),
            recovery: "`npm install` (ou pnpm/yarn/bun) traz tudo de volta.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![nm.clone()]),
            action: Action::remove(vec![nm]),
        });
    }

    for (id, rel, label, detail) in [
        ("node.npm", ".npm/_cacache", "Cache do npm", "Todo tarball de pacote já baixado pelo npm."),
        ("node.pnpm", "Library/pnpm/store", "Store do pnpm", "Store global de pacotes do pnpm."),
        ("node.yarn", "Library/Caches/Yarn", "Cache do Yarn", "Pacotes baixados pelo Yarn."),
        ("node.bun", ".bun/install/cache", "Cache do Bun", "Pacotes baixados pelo Bun."),
        ("node.deno", "Library/Caches/deno", "Cache do Deno", "Módulos remotos em cache do Deno."),
        ("node.nodegyp", "Library/Caches/node-gyp", "Headers do node-gyp", "Headers de cada versão de Node, usados para compilar módulos nativos."),
        ("node.vite", "Library/Caches/vite", "Cache do Vite", "Pré-bundle de dependências do Vite."),
        ("node.cypress", "Library/Caches/Cypress", "Binários do Cypress", "Uma cópia do Cypress por versão já usada."),
        ("node.playwright", "Library/Caches/ms-playwright", "Navegadores do Playwright", "Chromium, Firefox e WebKit baixados pelo Playwright."),
        ("node.puppeteer", ".cache/puppeteer", "Chromium do Puppeteer", "Cópias de Chromium baixadas pelo Puppeteer."),
        ("node.electron", "Library/Caches/electron", "Binários do Electron", "Uma cópia do Electron por versão já usada em build."),
    ] {
        let p = h.join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: id.into(),
            group: G_NODE,
            label: label.into(),
            detail: detail.into(),
            recovery: "Rebaixa sozinho quando algum projeto precisar.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }

    // Versões de Node no nvm: mantém a atual, oferece as outras.
    let current = run("node", &["-v"]).map(|v| v.trim().to_string());
    for v in children(&h.join(".nvm/versions/node")) {
        let ver = name_of(&v);
        if Some(&ver) == current.as_ref() {
            continue;
        }
        specs.push(Spec {
            id: format!("node.nvm.{ver}"),
            group: G_NODE,
            label: format!("Node {ver} (nvm)"),
            detail: format!(
                "Versão instalada pelo nvm, com os pacotes globais dela. Sua versão ativa {} \
                 não aparece nessa lista.",
                current.clone().unwrap_or_else(|| "atual".into())
            ),
            recovery: format!("`nvm install {ver}` reinstala."),
            risk: Risk::Low,
            size: SizeOf::Du(vec![v.clone()]),
            action: Action::remove(vec![v]),
        });
    }
}

// --- Docker ----------------------------------------------------------------

fn docker(specs: &mut Vec<Spec>) {
    let daemon_up = run("docker", &["info", "--format", "{{.ServerVersion}}"]).is_some();

    if daemon_up {
        // Números reais do daemon, em vez de chutar pelo tamanho do disco virtual.
        let df = run("docker", &["system", "df", "--format", "{{json .}}"]).unwrap_or_default();
        let mut reclaimable: Vec<(&str, u64)> = Vec::new();
        for line in df.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let kind = v["Type"].as_str().unwrap_or("");
            let bytes = parse_docker_size(v["Reclaimable"].as_str().unwrap_or("0B"));
            match kind {
                "Images" => reclaimable.push(("images", bytes)),
                "Build Cache" => reclaimable.push(("build", bytes)),
                "Local Volumes" => reclaimable.push(("volumes", bytes)),
                _ => {}
            }
        }
        let get = |k: &str| reclaimable.iter().find(|(n, _)| *n == k).map(|(_, b)| *b).unwrap_or(0);

        if get("images") > 0 {
            specs.push(Spec {
                id: "docker.images".into(),
                group: G_DOCKER,
                label: "Imagens e contêineres parados".into(),
                detail: "Toda imagem que não está em uso por um contêiner rodando, mais os \
                         contêineres já encerrados. Volumes não são tocados."
                    .into(),
                recovery: "`docker pull` / rebuild traz de volta.".into(),
                risk: Risk::Low,
                size: SizeOf::Fixed(get("images")),
                action: Action::exec(vec![cmd(&["docker", "system", "prune", "-a", "-f"])]),
            });
        }
        if get("build") > 0 {
            specs.push(Spec {
                id: "docker.build".into(),
                group: G_DOCKER,
                label: "Cache de build".into(),
                detail: "Camadas intermediárias do BuildKit.".into(),
                recovery: "Regenera no próximo build — que fica mais lento uma vez.".into(),
                risk: Risk::Safe,
                size: SizeOf::Fixed(get("build")),
                action: Action::exec(vec![cmd(&["docker", "builder", "prune", "-a", "-f"])]),
            });
        }
        if get("volumes") > 0 {
            let names = run("docker", &["volume", "ls", "-q", "-f", "dangling=true"])
                .unwrap_or_default()
                .lines()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            specs.push(Spec {
                id: "docker.volumes".into(),
                group: G_DOCKER,
                label: "Volumes sem contêiner".into(),
                detail: format!(
                    "CUIDADO: volume parado não é volume descartável — é aqui que mora o banco \
                     de dados local dos seus projetos. Afetados: {}.",
                    if names.is_empty() { "nenhum".into() } else { names }
                ),
                recovery: "Não volta. Você recria o banco do zero e roda as migrations.".into(),
                risk: Risk::Medium,
                size: SizeOf::Fixed(get("volumes")),
                action: Action::exec(vec![cmd(&["docker", "volume", "prune", "-a", "-f"])]),
            });
        }
    }

    // O Docker.raw não encolhe sozinho depois do prune, então vale mostrar.
    let raw = home().join("Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw");
    if full_disk_access() && raw.is_file() {
        specs.push(Spec {
            id: "docker.raw".into(),
            group: G_DOCKER,
            label: "Disco virtual do Docker (reset total)".into(),
            detail: format!(
                "O Docker.raw não devolve espaço sozinho depois de um prune. Apagar aqui zera \
                 TUDO: imagens, contêineres e volumes, inclusive os que estão em uso.{}",
                if daemon_up {
                    " Feche o Docker Desktop antes de rodar isso."
                } else {
                    ""
                }
            ),
            recovery: "O Docker recria o disco vazio ao abrir. Você repull e recria os volumes."
                .into(),
            risk: Risk::Medium,
            size: SizeOf::Du(vec![raw.clone()]),
            action: Action::remove(vec![raw]),
        });
    }
}

/// Converte "4.719GB" / "151.7MB" do `docker system df` em bytes.
fn parse_docker_size(s: &str) -> u64 {
    let s = s.trim();
    let digits: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let n: f64 = digits.parse().unwrap_or(0.0);
    let unit: String = s[digits.len()..].trim().to_ascii_uppercase();
    let mult = match unit.as_str() {
        "TB" | "TIB" => 1024f64.powi(4),
        "GB" | "GIB" => 1024f64.powi(3),
        "MB" | "MIB" => 1024f64.powi(2),
        "KB" | "KIB" => 1024.0,
        _ => 1.0,
    };
    (n * mult) as u64
}

// --- Builds & toolchains ---------------------------------------------------

fn build_artifacts(specs: &mut Vec<Spec>) {
    let h = home();
    let roots = dev_roots();

    // `target/` só é lixo de build se tiver um Cargo.toml do lado.
    for t in find_dirs(&roots, "target", 4) {
        if !t.with_file_name("Cargo.toml").exists() {
            continue;
        }
        specs.push(Spec {
            id: format!("build.target.{}", t.to_string_lossy()),
            group: G_BUILD,
            label: format!("target/ · {}", project_of(&t)),
            detail: t.to_string_lossy().to_string(),
            recovery: "`cargo build` recompila.".into(),
            risk: Risk::Safe,
            size: SizeOf::Du(vec![t.clone()]),
            action: Action::remove(vec![t]),
        });
    }

    for (id, rel, label, detail, recovery, risk) in [
        (
            "build.cargo.registry",
            ".cargo/registry",
            "Registry do Cargo",
            "Fontes e caches de todo crate já baixado.",
            "Rebaixa no próximo `cargo build`.",
            Risk::Safe,
        ),
        (
            "build.rustup.toolchains",
            ".rustup/toolchains",
            "Toolchains do Rust",
            "CUIDADO: inclui a toolchain que você usa hoje. Prefira `rustup toolchain list` e \
             remover só as antigas à mão.",
            "`rustup toolchain install` reinstala (~1 GB cada).",
            Risk::Medium,
        ),
        (
            "build.go",
            "go/pkg/mod",
            "Módulos do Go",
            "Cache de módulos baixados pelo Go.",
            "`go mod download` rebaixa.",
            Risk::Safe,
        ),
        (
            "build.gem",
            ".gem",
            "Gems do Ruby",
            "Gems instaladas fora do sistema.",
            "`bundle install` reinstala.",
            Risk::Low,
        ),
        (
            "build.pip",
            "Library/Caches/pip",
            "Cache do pip",
            "Wheels e tarballs baixados pelo pip.",
            "Rebaixa no próximo `pip install`.",
            Risk::Safe,
        ),
        (
            "build.uv",
            "Library/Caches/uv",
            "Cache do uv",
            "Cache de pacotes Python do uv.",
            "Rebaixa sozinho.",
            Risk::Safe,
        ),
        (
            "build.composer",
            ".composer/cache",
            "Cache do Composer",
            "Pacotes PHP baixados pelo Composer.",
            "`composer install` rebaixa.",
            Risk::Safe,
        ),
        (
            "build.dotnet",
            ".nuget/packages",
            "Pacotes NuGet",
            "Cache global de pacotes .NET.",
            "`dotnet restore` rebaixa.",
            Risk::Safe,
        ),
        (
            "build.homebrew",
            "Library/Caches/Homebrew",
            "Downloads do Homebrew",
            "Tarballs e bottles já instalados, guardados só para reinstalação offline.",
            "Rebaixa se você reinstalar a fórmula.",
            Risk::Safe,
        ),
    ] {
        let p = h.join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: id.into(),
            group: G_BUILD,
            label: label.into(),
            detail: detail.into(),
            recovery: recovery.into(),
            risk,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }
}

// --- Sistema ---------------------------------------------------------------

fn system(specs: &mut Vec<Spec>) {
    let h = home();

    let trash = h.join(".Trash");
    if live(&trash) {
        specs.push(Spec {
            id: "sys.trash".into(),
            group: G_SYS,
            label: "Lixeira".into(),
            detail: "O que já estava esperando pra ser apagado.".into(),
            recovery: "Não volta — mas você já tinha jogado fora.".into(),
            risk: Risk::Low,
            size: SizeOf::Du(vec![trash.clone()]),
            action: Action::empty(vec![trash]),
        });
    }

    for (id, rel, label, detail, recovery, risk) in [
        (
            "sys.wallpaper",
            "Library/Application Support/com.apple.wallpaper",
            "Papéis de parede em vídeo",
            "Os vídeos em 4K dos protetores de tela dinâmicos da Apple.",
            "Rebaixa se você selecionar o papel de parede de novo.",
            Risk::Safe,
        ),
        (
            "sys.mobileassets",
            "Library/Application Support/com.apple.mobileAssetDesktop",
            "Assets baixados do sistema",
            "Vozes, dicionários e modelos baixados sob demanda pelo macOS.",
            "Rebaixa quando o recurso for usado.",
            Risk::Safe,
        ),
        (
            "sys.logs",
            "Library/Logs",
            "Logs de aplicativos",
            "Logs e relatórios de crash dos apps do usuário.",
            "Não volta, e não faz falta fora de uma investigação de bug.",
            Risk::Low,
        ),
        (
            "sys.crashreports",
            "Library/Application Support/CrashReporter",
            "Relatórios de crash",
            "Histórico de travamentos de apps.",
            "Não volta.",
            Risk::Low,
        ),
        (
            "sys.quicklook",
            "Library/Caches/com.apple.QuickLook.thumbnailcache",
            "Miniaturas do Quick Look",
            "Cache de pré-visualização de arquivos no Finder.",
            "Regenera ao navegar pelas pastas.",
            Risk::Safe,
        ),
        (
            "sys.code",
            "Library/Application Support/Code/Cache",
            "Cache do VS Code",
            "Cache de rede e de renderização do VS Code.",
            "Regenera sozinho.",
            Risk::Safe,
        ),
        (
            "sys.codecached",
            "Library/Application Support/Code/CachedData",
            "Bytecode em cache do VS Code",
            "Bytecode V8 pré-compilado, uma pasta por versão do editor já instalada.",
            "Regenera sozinho — o primeiro start fica um pouco mais lento.",
            Risk::Safe,
        ),
        (
            "sys.slack",
            "Library/Application Support/Slack/Service Worker/CacheStorage",
            "Cache do Slack",
            "Anexos e imagens em cache do Slack.",
            "Rebaixa ao abrir as conversas.",
            Risk::Safe,
        ),
        (
            "sys.spotify",
            "Library/Application Support/Spotify/PersistentCache",
            "Cache do Spotify",
            "Músicas em cache (não inclui downloads offline marcados por você).",
            "Rebaixa ao tocar de novo.",
            Risk::Safe,
        ),
        (
            "sys.discord",
            "Library/Application Support/discord/Cache",
            "Cache do Discord",
            "Imagens e anexos em cache.",
            "Rebaixa sozinho.",
            Risk::Safe,
        ),
        (
            "sys.edge",
            "Library/Caches/Microsoft Edge",
            "Cache do Microsoft Edge",
            "Cache de navegação. Não mexe em senhas, histórico nem abas.",
            "Rebaixa navegando.",
            Risk::Safe,
        ),
        (
            "sys.chrome",
            "Library/Caches/Google/Chrome",
            "Cache do Chrome",
            "Cache de navegação. Não mexe em senhas, histórico nem abas.",
            "Rebaixa navegando.",
            Risk::Safe,
        ),
    ] {
        let p = h.join(rel);
        if !live(&p) {
            continue;
        }
        specs.push(Spec {
            id: id.into(),
            group: G_SYS,
            label: label.into(),
            detail: detail.into(),
            recovery: recovery.into(),
            risk,
            size: SizeOf::Du(vec![p.clone()]),
            action: Action::empty(vec![p]),
        });
    }

    // WhatsApp: só a mídia baixada. O ChatStorage.sqlite e o login ficam.
    let wa = h.join("Library/Group Containers/group.net.whatsapp.WhatsApp.shared/Message/Media");
    if full_disk_access() && live(&wa) {
        specs.push(Spec {
            id: "sys.whatsapp".into(),
            group: G_SYS,
            label: "Mídia do WhatsApp".into(),
            detail: "Fotos, vídeos e áudios baixados das conversas. As mensagens, os contatos e \
                     o seu login continuam intactos — só o arquivo de mídia sai."
                .into(),
            recovery: "Rebaixa por conversa se o remetente ainda tiver o arquivo. Se não tiver, \
                       some de vez."
                .into(),
            risk: Risk::Medium,
            size: SizeOf::Du(vec![wa.clone()]),
            action: Action::empty(vec![wa]),
        });
    }

    // Um alvo por app pra não jogar fora o cache de tudo de uma vez.
    let caches = h.join("Library/Caches");
    let mut generic: Vec<PathBuf> = Vec::new();
    for c in children(&caches) {
        let n = name_of(&c);
        // Os grandes já viraram alvo dedicado acima.
        let claimed = [
            "com.apple.dt.Xcode",
            "CocoaPods",
            "org.carthage.CarthageKit",
            "Homebrew",
            "node-gyp",
            "deno",
            "vite",
            "Yarn",
            "Cypress",
            "ms-playwright",
            "electron",
            "pip",
            "uv",
            "Google",
            "Microsoft Edge",
            "com.apple.QuickLook.thumbnailcache",
        ];
        if claimed.contains(&n.as_str()) || !c.is_dir() {
            continue;
        }
        generic.push(c);
    }
    if !generic.is_empty() {
        specs.push(Spec {
            id: "sys.caches".into(),
            group: G_SYS,
            label: "Caches restantes de apps".into(),
            detail: format!(
                "Tudo o que sobrou em ~/Library/Caches ({} pastas), fora os caches que já \
                 aparecem como item próprio nessa lista.",
                generic.len()
            ),
            recovery: "Cada app regenera o seu. Alguns pedem um restart.".into(),
            risk: Risk::Low,
            size: SizeOf::Du(generic.clone()),
            action: Action::remove(generic),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_sizes_parse() {
        assert_eq!(parse_docker_size("0B"), 0);
        assert_eq!(parse_docker_size("1KB"), 1024);
        assert_eq!(parse_docker_size("151.7MB"), (151.7 * 1024f64.powi(2)) as u64);
        assert_eq!(parse_docker_size("4.719GB"), (4.719 * 1024f64.powi(3)) as u64);
        assert_eq!(parse_docker_size("lixo"), 0);
    }

    #[test]
    fn catalogo_nao_tem_id_repetido() {
        let specs = build();
        let mut ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
        ids.sort();
        let total = ids.len();
        ids.dedup();
        assert_eq!(total, ids.len(), "ids duplicados quebram o lookup do clean()");
    }

    /// Smoke test: um Mac de desenvolvimento sempre tem algo pra limpar. Se
    /// isso zerar, algum caminho do catálogo quebrou e o app abre vazio.
    #[test]
    fn catalogo_encontra_alvos_nesta_maquina() {
        let specs = build();
        for s in &specs {
            println!("{:<28} {:<22} {}", s.id, s.group, s.label);
        }
        assert!(!specs.is_empty());
    }

    #[test]
    fn recomendado_nunca_inclui_o_que_nao_volta() {
        for spec in build() {
            if spec.risk == Risk::Medium && spec.id != "sys.trash" {
                assert!(
                    !is_recommended(&spec.id, spec.risk),
                    "{} não pode entrar nos recomendados",
                    spec.id
                );
            }
        }
        // Artefato por projeto é escolha do usuário, mesmo sendo Safe.
        assert!(!is_recommended("node.modules./x/y/node_modules", Risk::Safe));
        assert!(!is_recommended("build.target./x/y/target", Risk::Safe));
        // Mas o cache global do Cargo entra.
        assert!(is_recommended("build.cargo.registry", Risk::Safe));
    }

    #[test]
    fn catalogo_nunca_mira_a_home_inteira() {
        let h = home();
        for spec in build() {
            for p in spec.action.remove.iter().chain(spec.action.empty.iter()) {
                assert!(p != &h, "{} miraria a home inteira", spec.id);
                assert!(p.starts_with(&h) || p.starts_with("/Library"), "{}", spec.id);
            }
        }
    }
}
