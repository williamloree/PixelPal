//! Menu de soins, ouvert d'un clic sur le personnage : jauges, réserve de nourriture, caresse, panneau.
//! Dessiné en pixel art à taille 1, puis agrandi d'un facteur `u` (net, sans flou).

use crate::scene::{self, CREME, LAVANDE, NUIT, OR};
use crate::soins::{ALIMENTS, Soins};
use crate::toile::Toile;
use std::time::Instant;

const L: i32 = 104; // largeur en points
const LIGNE: i32 = 12;

pub struct Choix { pub cle: &'static str, libelle: String, actif: bool }

pub struct Menu {
    pub acteur: u64,
    pub ouvert: Instant,
    pub survol: Option<usize>,
    nom: String,
    soins: Soins,
    pub choix: Vec<Choix>,
}

impl Menu {
    pub fn new(acteur: u64, nom: &str, soins: &Soins) -> Menu {
        let mut m = Menu { acteur, ouvert: Instant::now(), survol: None, nom: nom.to_string(), soins: soins.clone(), choix: vec![] };
        m.maj(soins);
        m
    }

    /// Recalcule les jauges et les choix (la réserve a pu changer).
    pub fn maj(&mut self, soins: &Soins) {
        self.soins = soins.clone();
        self.choix.clear();
        if soins.mort {
            self.choix.push(Choix { cle: "ressusciter", libelle: "RESSUSCITER".into(), actif: true });
        } else {
            for (cle, nom) in ALIMENTS {
                let n = soins.stock(cle);
                let court = match cle { "pomme" => "POMME", "baie" => "BAIE", _ => "CHAMPIGNON" };
                let _ = nom;
                self.choix.push(Choix { cle, libelle: format!("{court} X{n}"), actif: n > 0 });
            }
            self.choix.push(Choix { cle: "caresser", libelle: "CARESSER".into(), actif: true });
        }
        self.choix.push(Choix { cle: "panneau", libelle: "PANNEAU".into(), actif: true });
    }

    fn haut_choix(&self) -> i32 { 4 + 11 + if self.soins.mort { 10 } else { 3 * 10 } + 4 }
    pub fn hauteur(&self) -> i32 { self.haut_choix() + self.choix.len() as i32 * LIGNE + 4 }
    pub fn largeur() -> i32 { L }

    /// Taille d'un point du menu à l'écran : la moitié de celle du personnage (texte net, pas trop gros).
    pub fn echelle(u: i32) -> i32 { (u / 2).max(2) }

    /// Choix sous le point (coordonnées en points du menu).
    pub fn choix_a(&self, y: i32) -> Option<usize> {
        let i = (y - self.haut_choix()).div_euclid(LIGNE);
        if y >= self.haut_choix() && (i as usize) < self.choix.len() { Some(i as usize) } else { None }
    }

    pub fn rendre(&self) -> Toile {
        let (w, h) = (L, self.hauteur());
        let mut t = Toile::new(w, h);
        // cadre de pierre aux coins coupés
        t.rect(NUIT, 1, 0, w - 2, h);
        t.rect(NUIT, 0, 1, w, h - 2);
        t.rect(0xFF231F3A, 1, 1, w - 2, h - 2);
        t.rect(0xFF4B4470, 2, 1, w - 4, 1);
        t.rect(0xFF17132A, 2, h - 2, w - 4, 1);
        for (x, y, c) in [(1, 1, 0xFF6FBF4F), (2, 1, 0xFF3F6B3A), (1, 2, 0xFF3F6B3A), (w - 2, h - 2, 0xFF6FBF4F), (w - 3, h - 2, 0xFF3F6B3A)] { t.rect(c, x, y, 1, 1); }
        let nom: String = self.nom.chars().take(14).collect::<String>().to_uppercase();
        scene::ecrire(&mut t, &nom, 5, 5, OR, None);
        let mut y = 4 + 11;
        if self.soins.mort {
            scene::ecrire(&mut t, "FANTÔME", 5, y, LAVANDE, None);
            y += 10;
        } else {
            for (lib, v, c) in [("VENTRE", self.soins.satiete, 0xFFF2C14E), ("SANTÉ", self.soins.sante, 0xFF7CE08A), ("JOIE", self.soins.joie, 0xFFFF7AD9)] {
                scene::ecrire(&mut t, lib, 5, y, CREME, None);
                let (bx, bw) = (44, w - 49);
                t.rect(NUIT, bx, y + 1, bw, 5);
                t.rect(0xFF3C3656, bx + 1, y + 2, bw - 2, 3);
                let n = ((bw - 2) as f32 * v / 100.0).round() as i32;
                let c = if v < 25.0 { 0xFFE0533E } else { c };
                if n > 0 { t.rect(c, bx + 1, y + 2, n, 3); t.rect(0x55FFFFFF, bx + 1, y + 2, n, 1); }
                y += 10;
            }
        }
        t.rect(0xFF4B4470, 4, y + 1, w - 8, 1);
        y = self.haut_choix();
        for (i, c) in self.choix.iter().enumerate() {
            if self.survol == Some(i) && c.actif { t.rect(0xFF4B2F7E, 2, y, w - 4, LIGNE); t.rect(OR, 2, y, 1, LIGNE); }
            let couleur = if !c.actif { 0xFF6E6896 } else if self.survol == Some(i) { 0xFFFFE9A8 } else { CREME };
            let ic = match c.cle {
                "pomme" | "baie" | "champi" => crate::sprites::icone_aliment(c.cle),
                "caresser" => { let mut x = Toile::new(8, 8); crate::sprites::coeur(&mut x, 1, 1, 2, 1.0); x.contour(NUIT); x }
                "ressusciter" => { let mut x = Toile::new(8, 8); crate::sprites::etoile(&mut x, 1, 3, 3); x.contour(NUIT); x }
                _ => { let x = scene::icone("chapeau", CREME); x }
            };
            t.coller(&ic, 4.0, (y + (LIGNE - ic.h) / 2) as f32, 1.0, false, if c.actif { 1.0 } else { 0.4 });
            scene::ecrire(&mut t, &c.libelle, 16, y + 3, couleur, None);
            y += LIGNE;
        }
        t
    }
}
