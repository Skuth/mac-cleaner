use std::path::PathBuf;

use serde::Serialize;

/// O que fazer para liberar o espaço de um alvo. As três etapas rodam nessa
/// ordem: a ferramenta oficial primeiro (ela sabe desregistrar o que apaga),
/// depois o que sobrou no disco.
#[derive(Clone, Debug, Default)]
pub struct Action {
    /// Comandos em sequência, onde a ferramenta limpa melhor que um `rm -rf`
    /// (simctl, docker, brew). Falha aqui não impede as etapas seguintes.
    pub exec: Vec<Vec<String>>,
    /// Caminhos removidos por inteiro.
    pub remove: Vec<PathBuf>,
    /// Diretórios esvaziados, mas mantidos existindo (apps recriam o conteúdo,
    /// mas alguns quebram se o diretório em si sumir).
    pub empty: Vec<PathBuf>,
}

impl Action {
    pub fn remove(paths: Vec<PathBuf>) -> Self {
        Self { remove: paths, ..Default::default() }
    }

    pub fn empty(paths: Vec<PathBuf>) -> Self {
        Self { empty: paths, ..Default::default() }
    }

    pub fn exec(cmds: Vec<Vec<String>>) -> Self {
        Self { exec: cmds, ..Default::default() }
    }
}

/// Quanto dói perder isso.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    /// Regenera sozinho, sem ação nenhuma da sua parte.
    Safe,
    /// Volta com um comando ou um download (build lento uma vez).
    Low,
    /// Some de vez, ou custa caro pra recuperar.
    Medium,
}

/// De onde vem o tamanho exibido.
#[derive(Clone, Debug)]
pub enum SizeOf {
    /// Roda `du -sk` nesses caminhos.
    Du(Vec<PathBuf>),
    /// Tamanho já conhecido (a própria ferramenta informou).
    Fixed(u64),
}

/// Um item do catálogo, antes de medir.
#[derive(Clone, Debug)]
pub struct Spec {
    pub id: String,
    pub group: &'static str,
    pub label: String,
    /// O que é isso.
    pub detail: String,
    /// Como volta depois de apagado.
    pub recovery: String,
    pub risk: Risk,
    pub size: SizeOf,
    pub action: Action,
}

/// Um item do catálogo, já medido, pronto pro front.
#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub id: String,
    pub group: &'static str,
    pub label: String,
    pub detail: String,
    pub recovery: String,
    pub risk: Risk,
    pub bytes: u64,
    /// Entra na seleção do botão "Marcar recomendados".
    pub recommended: bool,
}

/// Espaço do volume principal.
#[derive(Clone, Debug, Serialize)]
pub struct Disk {
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

/// Resultado de limpar um alvo.
#[derive(Clone, Debug, Serialize)]
pub struct CleanResult {
    pub id: String,
    pub label: String,
    pub freed: u64,
    pub error: Option<String>,
}
