//! A única parte que escreve no disco.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::{Emitter, State};

use crate::catalog::home;
use crate::scan::{read_disk, reveal_path};
use crate::types::{Action, CleanResult};
use crate::Catalog;

/// Recusa qualquer caminho que não seja claramente descartável.
///
/// As ações são montadas no backend, então isso não é defesa contra o
/// front-end — é a rede de proteção contra um bug meu no catálogo. Um
/// `join()` com string vazia vira a home inteira sem reclamar de nada.
fn guard(p: &Path) -> Result<(), String> {
    if !p.is_absolute() {
        return Err(format!("caminho relativo recusado: {}", p.display()));
    }
    if p.is_symlink() {
        return Err(format!("symlink recusado: {}", p.display()));
    }

    let h = home();
    let sob_home = p.starts_with(&h);
    if !sob_home && !p.starts_with("/Library/Developer") {
        return Err(format!("fora do escopo permitido: {}", p.display()));
    }

    // Raso demais: /Users, /Users/skuth, /Library — nunca é alvo legítimo.
    if p.components().count() < 4 {
        return Err(format!("caminho raso demais: {}", p.display()));
    }

    // Pastas que o usuário enche à mão. Nenhum cache mora nelas.
    const INTOCAVEL: [&str; 8] = [
        "Documents",
        "Desktop",
        "Downloads",
        "Pictures",
        "Movies",
        "Music",
        "Library",
        "Applications",
    ];
    if sob_home {
        if let Ok(rel) = p.strip_prefix(&h) {
            let parts: Vec<_> = rel.components().collect();
            if parts.len() == 1 && INTOCAVEL.contains(&parts[0].as_os_str().to_string_lossy().as_ref())
            {
                return Err(format!("pasta protegida: {}", p.display()));
            }
        }
    }
    Ok(())
}

fn remove(p: &Path) -> Result<(), String> {
    guard(p)?;
    if !p.exists() {
        return Ok(());
    }
    let r = if p.is_dir() {
        fs::remove_dir_all(p)
    } else {
        fs::remove_file(p)
    };
    r.map_err(|e| format!("{}: {e}", p.display()))
}

/// Apaga o conteúdo mas preserva o diretório — apps que guardam handle nele
/// quebram se o próprio diretório sumir.
fn empty(dir: &Path) -> Result<(), String> {
    guard(dir)?;
    if !dir.is_dir() {
        return Ok(());
    }
    let mut erros = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let p = entry.path();
        // Um item em uso não pode derrubar a limpeza dos outros.
        if let Err(e) = remove(&p) {
            erros.push(e);
        }
    }
    if erros.is_empty() {
        Ok(())
    } else {
        Err(erros.join("; "))
    }
}

fn apply(action: &Action) -> Option<String> {
    let mut erros = Vec::new();

    // A ferramenta oficial primeiro: ela desregistra o que apaga.
    for c in &action.exec {
        let Some((bin, args)) = c.split_first() else {
            continue;
        };
        match Command::new(bin).args(args).output() {
            Ok(out) if !out.status.success() => {
                let msg = String::from_utf8_lossy(&out.stderr);
                erros.push(format!("{}: {}", bin, msg.trim().lines().next().unwrap_or("falhou")));
            }
            Err(e) => erros.push(format!("{bin}: {e}")),
            _ => {}
        }
    }
    for p in &action.remove {
        if let Err(e) = remove(p) {
            erros.push(e);
        }
    }
    for p in &action.empty {
        if let Err(e) = empty(p) {
            erros.push(e);
        }
    }

    (!erros.is_empty()).then(|| erros.join(" | "))
}

#[tauri::command]
pub fn clean(
    app: tauri::AppHandle,
    catalog: State<Catalog>,
    items: Vec<(String, String)>,
) -> Vec<CleanResult> {
    // Copia as ações e solta o lock: a limpeza demora minutos e o scan não
    // pode ficar esperando por ela.
    let acoes: Vec<(String, String, Option<Action>)> = {
        let map = catalog.0.lock().unwrap();
        items
            .into_iter()
            .map(|(id, label)| {
                let a = map.get(&id).cloned();
                (id, label, a)
            })
            .collect()
    };

    let total = acoes.len();
    let mut results = Vec::with_capacity(total);

    for (i, (id, label, action)) in acoes.into_iter().enumerate() {
        let _ = app.emit("clean:progress", (i, total, label.clone()));

        let Some(action) = action else {
            results.push(CleanResult {
                id,
                label,
                freed: 0,
                error: Some("alvo desconhecido — rode o scan de novo".into()),
            });
            continue;
        };

        // Delta de espaço livre mede tudo do mesmo jeito, inclusive as ações
        // que rodam comando em vez de apagar caminho (docker, simctl).
        let antes = read_disk().free;
        let error = apply(&action);
        let depois = read_disk().free;

        results.push(CleanResult {
            id,
            label,
            freed: depois.saturating_sub(antes),
            error,
        });
    }

    let _ = app.emit("clean:progress", (total, total, String::new()));
    results
}

#[tauri::command]
pub fn reveal(path: String) {
    let p = PathBuf::from(path);
    if p.exists() {
        reveal_path(&p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_recusa_o_que_nao_deve_sumir() {
        let h = home();
        for ruim in [
            h.clone(),
            h.join("Documents"),
            h.join("Desktop"),
            h.join("Library"),
            PathBuf::from("/"),
            PathBuf::from("/Users"),
            PathBuf::from("/System"),
            PathBuf::from("relativo/qualquer"),
        ] {
            assert!(guard(&ruim).is_err(), "deveria recusar {}", ruim.display());
        }
    }

    #[test]
    fn guard_aceita_alvo_legitimo() {
        let h = home();
        for bom in [
            h.join("Library/Caches/algo"),
            h.join(".gradle/caches"),
            h.join("Documents/code/projeto/node_modules"),
            PathBuf::from("/Library/Developer/CoreSimulator/Caches"),
        ] {
            assert!(guard(&bom).is_ok(), "deveria aceitar {}", bom.display());
        }
    }

    #[test]
    fn empty_preserva_o_diretorio() {
        let base = home().join("Library/Caches/mac-cleaner-test");
        let _ = fs::create_dir_all(base.join("sub/mais/fundo"));
        fs::write(base.join("arquivo.txt"), b"x").unwrap();

        empty(&base).unwrap();

        assert!(base.is_dir(), "o diretório em si tem que continuar existindo");
        assert_eq!(fs::read_dir(&base).unwrap().count(), 0, "e vazio");
        let _ = fs::remove_dir_all(&base);
    }
}
