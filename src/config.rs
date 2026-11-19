//! Profil, contacts et groupes, enregistrés dans %APPDATA%\PixelPal[-profil]\pixelpal.json.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Membre {
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Groupe {
    pub id: String,
    pub nom: String,
    pub membres: Vec<Membre>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub id: String,
    pub nom: String,
    pub look: String,
    #[serde(default)]
    pub groupes: Vec<Groupe>,
    #[serde(default)]
    pub contacts: Vec<Membre>,
    #[serde(skip)]
    pub fichier: PathBuf,
}

pub fn dossier(profil: &Option<String>) -> PathBuf {
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    base.join(match profil { Some(p) => format!("PixelPal-{p}"), None => "PixelPal".into() })
}

pub fn id_aleatoire() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::new();
    for _ in 0..2 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

impl Config {
    pub fn charger(profil: &Option<String>) -> Config {
        let dir = dossier(profil);
        let _ = std::fs::create_dir_all(&dir);
        let fichier = dir.join("pixelpal.json");
        let mut c: Config = std::fs::read_to_string(&fichier).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if c.id.is_empty() { c.id = id_aleatoire(); }
        if c.nom.is_empty() {
            c.nom = match profil { Some(p) => format!("Testeur {p}"), None => std::env::var("USERNAME").unwrap_or_else(|_| "Moi".into()) };
        }
        if c.look.is_empty() { c.look = if profil.is_some() { "bleu".into() } else { "aventurier".into() }; }
        c.fichier = fichier;
        c.sauver();
        c
    }

    pub fn sauver(&self) {
        if let Ok(s) = serde_json::to_string_pretty(self) { let _ = std::fs::write(&self.fichier, s); }
    }
}
