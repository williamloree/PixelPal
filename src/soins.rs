//! Côté Tamagotchi : satiété, santé, joie, réserve de nourriture et de parachutes.
//! Le temps ne compte que quand l'appli est ouverte. Un repas toutes les 3 h environ ;
//! sans manger, il meurt au bout d'environ 12 h d'utilisation (6 h pour avoir faim, 6 h pour s'éteindre).

use serde::{Deserialize, Serialize};

const H: f32 = 3600.0;
pub const ALIMENTS: [(&str, &str); 3] = [("pomme", "Pomme"), ("baie", "Baie féerique"), ("champi", "Champignon lumineux")];

/// Effet d'un aliment : (satiété, joie, santé).
pub fn effet(cle: &str) -> (f32, f32, f32) {
    match cle {
        "pomme" => (30.0, 5.0, 0.0),
        "baie" => (15.0, 20.0, 0.0),
        _ => (45.0, 0.0, 10.0),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Soins {
    pub satiete: f32,
    pub sante: f32,
    pub joie: f32,
    pub age: f64,
    pub record: f64,
    pub mort: bool,
    pub pomme: u32,
    pub baie: u32,
    pub champi: u32,
    pub parachutes: u32,
    pub jardin: f32,      // secondes accumulées avant la prochaine récolte
    pub atelier: f32,     // secondes accumulées avant le prochain parachute
    pub resurrections: u32,
}

impl Default for Soins {
    fn default() -> Self {
        Soins { satiete: 80.0, sante: 100.0, joie: 70.0, age: 0.0, record: 0.0, mort: false, pomme: 3, baie: 2, champi: 1,
            parachutes: 2, jardin: 0.0, atelier: 0.0, resurrections: 0 }
    }
}

/// Ce qui vient de se passer (pour que le personnage le dise ou le montre).
pub enum Evenement { Recolte(&'static str), Parachute, Faim, Mort }

impl Soins {
    pub fn stock(&self, cle: &str) -> u32 { match cle { "pomme" => self.pomme, "baie" => self.baie, _ => self.champi } }
    fn stock_mut(&mut self, cle: &str) -> &mut u32 { match cle { "pomme" => &mut self.pomme, "baie" => &mut self.baie, _ => &mut self.champi } }
    pub fn total(&self) -> u32 { self.pomme + self.baie + self.champi }

    /// Avance le temps de `dt` secondes (appli ouverte). `vitesse` accélère tout (tests, démo).
    pub fn maj(&mut self, dt: f32, vitesse: f32, hasard: f32) -> Vec<Evenement> {
        let mut ev = vec![];
        if self.mort { return ev; }
        let dt = dt * vitesse;
        self.age += dt as f64;
        self.record = self.record.max(self.age);
        let avant = self.satiete;
        self.satiete = (self.satiete - dt * 100.0 / (6.0 * H)).max(0.0);
        self.joie = (self.joie - dt * 100.0 / (8.0 * H)).max(0.0);
        if avant >= 25.0 && self.satiete < 25.0 { ev.push(Evenement::Faim); }
        if self.satiete <= 0.0 {
            self.sante -= dt * 100.0 / (6.0 * H);
        } else if self.satiete > 50.0 {
            self.sante = (self.sante + dt * 100.0 / (2.0 * H)).min(100.0);
        }
        if self.sante <= 0.0 {
            self.sante = 0.0;
            self.mort = true;
            ev.push(Evenement::Mort);
        }
        // le jardin donne un aliment toutes les 40 min, l'atelier un parachute toutes les 2 h
        self.jardin += dt;
        if self.jardin >= 40.0 * 60.0 {
            self.jardin = 0.0;
            if self.total() < 15 {
                let cle = if hasard < 0.45 { "pomme" } else if hasard < 0.8 { "baie" } else { "champi" };
                *self.stock_mut(cle) += 1;
                ev.push(Evenement::Recolte(cle));
            }
        }
        self.atelier += dt;
        if self.atelier >= 2.0 * H {
            self.atelier = 0.0;
            if self.parachutes < 3 { self.parachutes += 1; ev.push(Evenement::Parachute); }
        }
        ev
    }

    /// Donne un aliment de la réserve ; renvoie faux s'il n'y en a plus (ou s'il est mort).
    pub fn nourrir(&mut self, cle: &str) -> bool {
        if self.mort || self.stock(cle) == 0 { return false; }
        *self.stock_mut(cle) -= 1;
        let (s, j, h) = effet(cle);
        self.satiete = (self.satiete + s).min(100.0);
        self.joie = (self.joie + j).min(100.0);
        self.sante = (self.sante + h).min(100.0);
        true
    }

    pub fn caresser(&mut self) { if !self.mort { self.joie = (self.joie + 10.0).min(100.0); } }

    /// Chute sans parachute.
    pub fn ecrasement(&mut self) {
        if self.mort { return; }
        self.sante = (self.sante - 15.0).max(1.0);
        self.joie = (self.joie - 10.0).max(0.0);
    }

    pub fn prendre_parachute(&mut self) -> bool {
        if self.parachutes == 0 { return false; }
        self.parachutes -= 1;
        true
    }

    pub fn ressusciter(&mut self) {
        self.mort = false;
        self.sante = 60.0;
        self.satiete = 60.0;
        self.joie = 50.0;
        self.age = 0.0;
        self.resurrections += 1;
    }

    pub fn humeur(&self) -> &'static str {
        if self.mort { "Fantôme" } else if self.satiete <= 0.0 { "Affamé, il s'affaiblit !" } else if self.satiete < 25.0 { "A faim" }
        else if self.joie < 20.0 { "Triste" } else if self.joie > 70.0 && self.satiete > 60.0 { "Radieux" } else { "Bien" }
    }
}

/// Âge lisible (« 3 h 12 », « 2 j 4 h »).
pub fn duree(s: f64) -> String {
    let m = (s / 60.0) as u64;
    if m < 60 { format!("{m} min") } else if m < 24 * 60 { format!("{} h {:02}", m / 60, m % 60) } else { format!("{} j {} h", m / 1440, m / 60 % 24) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meurt_apres_douze_heures_sans_manger() {
        let mut s = Soins { satiete: 100.0, ..Default::default() };
        let mut mort = false;
        for _ in 0..(11 * 60) { s.maj(60.0, 1.0, 0.5); }
        assert!(!s.mort, "trop tôt : santé {}", s.sante);
        for _ in 0..(2 * 60) { if s.maj(60.0, 1.0, 0.5).iter().any(|e| matches!(e, Evenement::Mort)) { mort = true; } }
        assert!(s.mort && mort);
        assert!(!s.nourrir("pomme"));
        s.ressusciter();
        assert!(!s.mort && s.age == 0.0);
    }

    #[test]
    fn manger_et_recolter() {
        let mut s = Soins::default();
        let n = s.pomme;
        assert!(s.nourrir("pomme"));
        assert_eq!(s.pomme, n - 1);
        let total = s.total();
        let ev = s.maj(40.0 * 60.0, 1.0, 0.9);
        assert!(ev.iter().any(|e| matches!(e, Evenement::Recolte("champi"))));
        assert_eq!(s.total(), total + 1);
    }
}
