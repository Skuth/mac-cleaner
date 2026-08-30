//! Medição. Nada aqui escreve no disco.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tauri::{Emitter, State};

use crate::catalog;
use crate::types::{Disk, SizeOf, Target};
use crate::Catalog;

/// Tamanho em bytes de um conjunto de caminhos, via `du -sk`.
///
/// `du` erra o status quando *qualquer* caminho falha, então o status é
/// ignorado de propósito — o que deu certo já veio no stdout.
pub fn du(paths: &[PathBuf]) -> u64 {
    let existing: Vec<&PathBuf> = paths.iter().filter(|p| p.exists()).collect();
    if existing.is_empty() {
        return 0;
    }
    let mut cmd = Command::new("du");
    cmd.arg("-sk");
    for p in &existing {
        cmd.arg(p);
    }
    let Ok(out) = cmd.output() else {
        return 0;
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter_map(|kb| kb.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .sum()
}

/// Quantos `du` rodam ao mesmo tempo. Alto demais só faz o SSD engasgar.
fn parallelism() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4)
}

#[tauri::command]
pub fn scan(app: tauri::AppHandle, catalog_state: State<Catalog>) -> Vec<Target> {
    let specs = catalog::build();
    let total = specs.len();

    // `du` é I/O puro: paralelizar corta o scan de minutos pra segundos.
    let sizes = std::sync::Mutex::new(vec![0u64; total]);
    let cursor = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);

    std::thread::scope(|s| {
        for _ in 0..parallelism() {
            s.spawn(|| loop {
                let i = cursor.fetch_add(1, Ordering::Relaxed);
                if i >= total {
                    break;
                }
                let bytes = match &specs[i].size {
                    SizeOf::Fixed(b) => *b,
                    SizeOf::Du(paths) => du(paths),
                };
                sizes.lock().unwrap()[i] = bytes;
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = app.emit("scan:progress", (n, total, specs[i].label.clone()));
            });
        }
    });

    let sizes = sizes.into_inner().unwrap();

    let mut map = catalog_state.0.lock().unwrap();
    map.clear();

    let mut targets: Vec<Target> = specs
        .into_iter()
        .zip(sizes)
        // Alvo vazio só polui a tela.
        .filter(|(_, bytes)| *bytes > 1024 * 1024)
        .map(|(spec, bytes)| {
            map.insert(spec.id.clone(), spec.action);
            Target {
                recommended: catalog::is_recommended(&spec.id, spec.risk),
                id: spec.id,
                group: spec.group,
                label: spec.label,
                detail: spec.detail,
                recovery: spec.recovery,
                risk: spec.risk,
                bytes,
            }
        })
        .collect();

    targets.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    targets
}

/// Espaço livre do volume de dados — a métrica que o usuário realmente vê.
#[tauri::command]
pub fn disk_usage() -> Disk {
    read_disk()
}

pub fn read_disk() -> Disk {
    // `df -k` na raiz reporta o container APFS inteiro, que é o número do Finder.
    let out = Command::new("df")
        .args(["-k", "/System/Volumes/Data"])
        .output()
        .ok();
    let text = out
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let fields: Vec<u64> = text
        .lines()
        .nth(1)
        .map(|l| {
            l.split_whitespace()
                .skip(1)
                .take(3)
                .filter_map(|f| f.parse::<u64>().ok())
                .collect()
        })
        .unwrap_or_default();

    if fields.len() < 3 {
        return Disk { total: 0, free: 0, used: 0 };
    }
    Disk {
        total: fields[0] * 1024,
        used: fields[1] * 1024,
        free: fields[2] * 1024,
    }
}

/// Abre o caminho no Finder, pra conferir antes de apagar.
pub fn reveal_path(p: &Path) {
    let _ = Command::new("open").arg("-R").arg(p).status();
}
