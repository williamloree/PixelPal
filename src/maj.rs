//! Mise à jour automatique depuis les releases GitHub.
//!
//! Seul l'exécutable compilé par le workflow (variable PIXELPAL_CI) se met à jour : une version
//! compilée à la main reste telle quelle. Les téléchargements passent par curl.exe (fourni avec Windows).
//! Dépôt privé : il faut un jeton GitHub en lecture, dans la variable PIXELPAL_GITHUB_TOKEN
//! ou dans %APPDATA%\PixelPal\github-token.txt.

use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub const DEPOT: &str = "williamloree/PixelPal";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const SANS_FENETRE: u32 = 0x0800_0000;

pub fn active() -> bool { option_env!("PIXELPAL_CI").is_some() }

pub struct Nouvelle { pub version: String, url: String, url_api: String }

fn jeton() -> Option<String> {
    std::env::var("PIXELPAL_GITHUB_TOKEN").ok()
        .or_else(|| std::fs::read_to_string(crate::config::dossier(&None).join("github-token.txt")).ok())
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn curl(url: &str, accept: &str, sortie: Option<&PathBuf>) -> Result<Vec<u8>, String> {
    let mut c = Command::new("curl.exe");
    c.args(["-sSfL", "--max-time", "300", "-H", "User-Agent: PixelPal", "-H", &format!("Accept: {accept}")]);
    if let Some(j) = jeton() { c.args(["-H", &format!("Authorization: Bearer {j}")]); }
    if let Some(f) = sortie { c.arg("-o").arg(f); }
    let o = c.arg(url).creation_flags(SANS_FENETRE).output().map_err(|e| format!("curl introuvable ({e})"))?;
    if !o.status.success() {
        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
        return Err(if err.contains("404") && jeton().is_none() { "dépôt privé ou sans release : jeton GitHub requis".into() } else { err });
    }
    Ok(o.stdout)
}

fn numeros(v: &str) -> Vec<u64> { v.trim_start_matches('v').split('.').map(|x| x.parse().unwrap_or(0)).collect() }

/// Cherche une version plus récente que la nôtre.
pub fn verifier() -> Result<Option<Nouvelle>, String> {
    let brut = curl(&format!("https://api.github.com/repos/{DEPOT}/releases/latest"), "application/vnd.github+json", None)?;
    let r: Value = serde_json::from_slice(&brut).map_err(|_| "réponse GitHub illisible")?;
    let tag = r["tag_name"].as_str().ok_or("release sans numéro")?;
    if numeros(tag) <= numeros(VERSION) { return Ok(None); }
    let a = r["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"] == "pixelpal.exe")).ok_or("release sans pixelpal.exe")?;
    Ok(Some(Nouvelle {
        version: tag.trim_start_matches('v').to_string(),
        url: a["browser_download_url"].as_str().unwrap_or("").into(),
        url_api: a["url"].as_str().unwrap_or("").into(),
    }))
}

fn exe() -> PathBuf { std::env::current_exe().expect("chemin de l'exécutable") }
fn ancien() -> PathBuf { exe().with_file_name("pixelpal.old.exe") }

/// Télécharge la nouvelle version et la met à la place de l'exécutable en cours
/// (Windows permet de renommer un exécutable qui tourne).
pub fn installer(n: &Nouvelle) -> Result<(), String> {
    let neuf = exe().with_file_name("pixelpal.new.exe");
    let _ = std::fs::remove_file(&neuf);
    // avec un jeton (dépôt privé), on passe par l'API ; sinon le lien public suffit
    if jeton().is_some() { curl(&n.url_api, "application/octet-stream", Some(&neuf))?; } else { curl(&n.url, "application/octet-stream", Some(&neuf))?; }
    let octets = std::fs::read(&neuf).map_err(|e| e.to_string())?;
    if octets.len() < 1_000_000 || !octets.starts_with(b"MZ") {
        let _ = std::fs::remove_file(&neuf);
        return Err("fichier téléchargé invalide".into());
    }
    let _ = std::fs::remove_file(ancien());
    std::fs::rename(exe(), ancien()).map_err(|e| format!("impossible de remplacer l'exécutable ({e})"))?;
    if let Err(e) = std::fs::rename(&neuf, exe()) {
        let _ = std::fs::rename(ancien(), exe());
        return Err(format!("installation impossible ({e})"));
    }
    Ok(())
}

/// Relance le (nouvel) exécutable avec les mêmes arguments.
pub fn relancer() {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--apres-maj").collect();
    let _ = Command::new(exe()).args(args).arg("--apres-maj").spawn();
}

/// Au démarrage : supprime l'ancienne version (elle peut encore tourner quelques secondes).
pub fn nettoyer() {
    std::thread::spawn(|| for _ in 0..10 {
        if !ancien().exists() || std::fs::remove_file(ancien()).is_ok() { return; }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}
