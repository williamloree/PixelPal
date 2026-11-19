//! Simulation : écrans, passages d'un écran à l'autre, acteurs (le personnage et les visiteurs), décors.
//! Toutes les coordonnées sont en pixels physiques de l'écran virtuel de Windows.

use crate::soins::{Evenement, Soins};
use std::collections::{HashMap, VecDeque};

pub const U_BASE: f32 = 3.0; // taille d'un « gros pixel » à 100 %

#[derive(Clone, Debug)]
pub struct Ecran {
    pub id: usize,
    pub nom: String,
    pub x: f32, pub y: f32, pub r: f32, pub b: f32,
    pub sol: f32,       // bas de la zone de travail (au-dessus de la barre des tâches)
    pub gx: f32, pub dx: f32, // limites de promenade
    pub principal: bool,
}

#[derive(Clone, Debug)]
pub enum TypeLien { Cote { sens: i32, x: f32 }, Bas { x0: f32, x1: f32 }, Haut { x0: f32, x1: f32 } }

#[derive(Clone, Debug)]
pub struct Lien { pub de: usize, pub vers: usize, pub t: TypeLien }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TypeObjet { Echelle, Trou, Portail, Tombe }

/// Ce sur quoi le personnage se tient.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Appui { Sol, Fenetre(isize), Plafond, Air }

/// Partie visible du bord supérieur d'une fenêtre ouverte : il peut marcher dessus.
#[derive(Clone, Debug)]
pub struct Plateforme { pub hwnd: isize, pub x0: f32, pub x1: f32, pub y: f32, pub gauche: f32 }

#[derive(Clone, Debug)]
pub struct Objet {
    pub id: u64,
    pub t: TypeObjet,
    pub x: f32, pub y: f32,           // trou, portail : centre et sol
    pub y_haut: f32, pub y_bas: f32,  // échelle
    pub vers_bas: bool,
    pub p: f32, pub cible: f32,
    pub vie: f32,
}

impl Objet {
    pub fn alpha(&self) -> f32 { if self.vie < 0.0 { (1.0 + self.vie / 2.0).max(0.0) } else { 1.0 } }
}

#[derive(Clone, Debug)]
pub struct Bulle { pub titre: String, pub texte: String, pub duree: Option<f32>, pub cliquable: bool }

/// Ce que la simulation demande au reste du programme.
pub enum Sortie {
    Menu(u64),                  // clic sur le personnage : menu de soins
    Envoyer(u64),               // lancer l'envoi n° … (quand le personnage passe le portail)
    AccuserLecture(serde_json::Value),
}

#[derive(Clone, Copy)]
enum Cond { Lu, Resultat }

enum Action {
    Marcher(f32),
    Attendre(f32, &'static str),
    AttendreQue(Cond, &'static str),
    Grimper(f32, f32),
    Tomber(f32),
    Sauter { x: f32, y: f32, t: f32, x0: f32, y0: f32 },
    Chute { vx: f32, vy: f32, y0: f32, para: bool },
    Appui(Appui),
    Renverser(bool),
    Couche(bool),
    Ranger,
    Nourrir(&'static str),
    Caresse,
    Mourir,
    Echelle { x: f32, y_haut: f32, y_bas: f32, vers_bas: bool, id: u64 },
    Creuser { x: f32, y: f32, id: u64 },
    OuvrirPortail(f32),
    RouvrirPortail,
    EntrerPortail { t: f32, x0: f32 },
    SortirPortail { dist: f32, t: f32 },
    FermerPortail,
    Ecran(usize),
    PoserY(f32),
    Sens(i32),
    Lettre(bool),
    Transit(bool),
    Occupe(bool),
    Bulle(Option<Bulle>),
    Teleporter { ecran: usize, x: f32 },
    LancerEnvoi(u64),
    DireResultat,
    Supprimer,
}

pub struct Acteur {
    pub id: u64,
    pub nom: String,
    pub look: String,
    pub x: f32, pub y: f32,
    pub sens: i32,
    pub anim: &'static str,
    pub anim_t: f32,
    pub visible: bool,
    pub echelle: f32,
    pub ecran: usize,
    pub visiteur: bool,
    pub bulle: Option<Bulle>,
    pub msg: Option<serde_json::Value>,
    file: VecDeque<Action>,
    action: Option<Action>,
    neuf: bool,
    portail: Option<u64>,
    lettre: bool,
    transit: bool,
    occupe: bool,
    lu: bool,
    resultat: Option<String>,
    vy: f32,
    supprimer: bool,
    pub appui: Appui,
    pub renverse: bool,   // au plafond, tête en bas
    pub couche: bool,     // allongé (KO)
    pub porte: bool,      // tenu par la souris
    pub parachute: bool,
    pub fantome: bool,
    pub etoiles: f32,     // secondes d'étourdissement restantes
    pub coeurs: f32,      // secondes de petits cœurs restantes
    dernier_objet: Option<u64>, // échelle ou trou à ranger après usage
}

impl Acteur {
    fn new(id: u64, nom: &str, look: &str, x: f32, y: f32, ecran: usize) -> Self {
        Acteur { id, nom: nom.into(), look: look.into(), x, y, sens: 1, anim: "idle", anim_t: 0.0, visible: true, echelle: 1.0, ecran,
            visiteur: false, bulle: None, msg: None, file: VecDeque::new(), action: None, neuf: false, portail: None, lettre: false,
            transit: false, occupe: false, lu: false, resultat: None, vy: 0.0, supprimer: false, appui: Appui::Sol, renverse: false,
            couche: false, porte: false, parachute: false, fantome: false, etoiles: 0.0, coeurs: 0.0, dernier_objet: None }
    }
    fn au_repos(&mut self) {
        self.appui = Appui::Sol; self.renverse = false; self.couche = false; self.porte = false; self.parachute = false;
    }
    fn anim(&mut self, a: &'static str) { if self.anim != a { self.anim = a; self.anim_t = 0.0; } }
    fn marche(&self) -> &'static str { if self.lettre { "carry" } else { "walk" } }
}

pub const MOI: u64 = 1;

pub struct Monde {
    pub ecrans: Vec<Ecran>,
    pub liens: Vec<Lien>,
    pub acteurs: Vec<Acteur>,
    pub objets: Vec<Objet>,
    pub sorties: Vec<Sortie>,
    pub plateformes: Vec<Plateforme>,
    pub soins: Soins,
    pub vitesse_soins: f32, // 1 = temps réel d'utilisation
    pub k: f32, // facteur d'échelle (DPI de l'écran principal)
    pub temps: f32,
    rappel_faim: f32,
    seq: u64,
    alea: u64,
}

fn clamp(v: f32, a: f32, b: f32) -> f32 { if a > b { (a + b) / 2.0 } else { v.max(a).min(b) } }

impl Monde {
    pub fn new(nom: &str, look: &str) -> Self {
        let graine = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        let mut m = Monde { ecrans: vec![], liens: vec![], acteurs: vec![], objets: vec![], sorties: vec![], plateformes: vec![],
            soins: Soins::default(), vitesse_soins: 1.0, k: 1.0, temps: 0.0, rappel_faim: 0.0, seq: 10, alea: graine | 1 };
        m.acteurs.push(Acteur::new(MOI, nom, look, 0.0, 0.0, 0));
        m
    }

    pub fn u(&self) -> i32 { (U_BASE * self.k).round().max(1.0) as i32 }
    fn px(&self, v: f32) -> f32 { v * self.k }

    pub fn hasard(&mut self) -> f32 {
        self.alea ^= self.alea << 13; self.alea ^= self.alea >> 7; self.alea ^= self.alea << 17;
        (self.alea >> 40) as f32 / (1u64 << 24) as f32
    }
    fn entre(&mut self, a: f32, b: f32) -> f32 { a + (b - a) * self.hasard() }
    fn nid(&mut self) -> u64 { self.seq += 1; self.seq }

    pub fn ecran(&self, id: usize) -> &Ecran { self.ecrans.iter().find(|e| e.id == id).unwrap_or(&self.ecrans[0]) }

    pub fn moi(&mut self) -> &mut Acteur { self.acteurs.iter_mut().find(|a| a.id == MOI).unwrap() }

    /// (Re)définit les écrans ; au premier appel, le personnage arrive par un portail.
    pub fn definir_ecrans(&mut self, ecrans: Vec<Ecran>, k: f32) {
        let premier = self.ecrans.is_empty();
        self.k = k;
        // tolérance large : avec des échelles d'affichage différentes, Windows laisse parfois un petit écart
        self.liens = calculer_liens(&ecrans, self.px(40.0));
        self.ecrans = ecrans;
        self.objets.clear();
        let marge = self.px(60.0);
        for e in &mut self.ecrans { e.gx += marge; e.dx -= marge; }
        let principal = self.ecrans.iter().find(|e| e.principal).unwrap_or(&self.ecrans[0]).clone();
        let ids: Vec<usize> = self.ecrans.iter().map(|e| e.id).collect();
        for a in &mut self.acteurs {
            // on repose tout le monde au sol (les écrans ont pu bouger)
            let e = if ids.contains(&a.ecran) && !premier { a.ecran } else { principal.id };
            a.ecran = e;
            a.file.clear(); a.action = None; a.transit = false; a.visible = true; a.echelle = 1.0;
            a.occupe = false; a.lettre = a.visiteur; a.portail = None;
            a.au_repos();
        }
        let ecrans = self.ecrans.clone();
        for a in &mut self.acteurs {
            let e = ecrans.iter().find(|e| e.id == a.ecran).unwrap();
            a.y = e.sol;
            a.x = clamp(a.x, e.gx, e.dx);
            if a.visiteur { a.file.push_back(Action::AttendreQue(Cond::Lu, "deliver")); a.file.extend(Self::depart_visiteur()); }
        }
        if self.soins.mort {
            // il était déjà mort : sa tombe et son fantôme l'attendent
            let m = self.moi();
            m.fantome = true;
            let (x, y) = (m.x, m.y);
            let id = self.nid();
            self.objets.push(Objet { id, t: TypeObjet::Tombe, x, y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 1.0, cible: 1.0, vie: f32::MAX });
            return;
        }
        if premier {
            let x = (principal.gx + principal.dx) / 2.0;
            let m = self.moi();
            m.x = x; m.visible = false; m.ecran = principal.id;
            let dx = self.px(40.0);
            let m = self.moi();
            m.x -= dx;
            m.file.extend([Action::Occupe(true), Action::OuvrirPortail(0.0), Action::SortirPortail { dist: dx * 2.0, t: 0.0 }, Action::FermerPortail,
                Action::Bulle(Some(Bulle { titre: String::new(), texte: "Coucou ! Clique sur moi.".into(), duree: Some(4.0), cliquable: false })),
                Action::Attendre(1.5, "wave"), Action::Occupe(false)]);
        }
    }

    fn chemin(&self, de: usize, vers: usize) -> Option<Vec<Lien>> {
        if de == vers { return Some(vec![]); }
        let mut prec: HashMap<usize, Option<Lien>> = HashMap::from([(de, None)]);
        let mut q = VecDeque::from([de]);
        while let Some(c) = q.pop_front() {
            for l in self.liens.iter().filter(|l| l.de == c) {
                if prec.contains_key(&l.vers) { continue; }
                prec.insert(l.vers, Some(l.clone()));
                if l.vers == vers {
                    let mut ch = vec![];
                    let mut k = vers;
                    while let Some(Some(l)) = prec.get(&k) { ch.insert(0, l.clone()); k = l.de; }
                    return Some(ch);
                }
                q.push_back(l.vers);
            }
        }
        None
    }

    /// Prépare le trajet du personnage jusqu'à l'écran `cible`.
    /// Il pose une échelle neuve à chaque fois et la range une fois passé ; les trous se rebouchent.
    fn aller_a(&mut self, ai: usize, cible: usize) {
        let de = self.acteurs[ai].ecran;
        let Some(ch) = self.chemin(de, cible) else { return self.voyage_portail(ai, cible); };
        if ch.is_empty() { return; }
        let mut l_actions = vec![Action::Transit(true)];
        let mut x = self.acteurs[ai].x;
        let p = |m: &Monde, v: f32| m.px(v);
        for l in ch {
            let (ea, eb) = (self.ecran(l.de).clone(), self.ecran(l.vers).clone());
            let echelle = |x: f32, y_haut: f32, y_bas: f32, vers_bas: bool| Action::Echelle { x, y_haut, y_bas, vers_bas, id: 0 };
            match l.t {
                TypeLien::Cote { sens: s, x: bord } => {
                    let sf = s as f32;
                    let (fa, fb) = (ea.sol, eb.sol);
                    let diff = fb - fa;
                    if diff.abs() <= p(self, 8.0) {
                        l_actions.extend([Action::Marcher(bord + sf * p(self, 70.0)), Action::PoserY(fb), Action::Ecran(eb.id)]);
                    } else if diff < 0.0 {
                        // l'écran d'arrivée est plus haut : échelle contre le bord, côté départ
                        let lx = bord - sf * p(self, 30.0);
                        l_actions.extend([Action::Marcher(lx - sf * p(self, 45.0)), Action::Sens(s), echelle(lx, fb - p(self, 30.0), fa, false),
                            Action::Marcher(lx), Action::Grimper(fb, lx), Action::Ecran(eb.id), Action::Marcher(bord + sf * p(self, 70.0)), Action::Ranger]);
                    } else if diff <= p(self, 160.0) {
                        // petite marche : il saute
                        l_actions.extend([Action::Marcher(bord - sf * p(self, 15.0)),
                            Action::Sauter { x: bord + sf * p(self, 80.0), y: fb, t: -1.0, x0: 0.0, y0: 0.0 },
                            Action::Ecran(eb.id), Action::Attendre(0.3, "land")]);
                    } else {
                        // grande descente : il pose une échelle depuis le haut, de l'autre côté du bord
                        let lx = bord + sf * p(self, 30.0);
                        l_actions.extend([Action::Marcher(bord - sf * p(self, 20.0)), Action::Sens(s), echelle(lx, fa - p(self, 30.0), fb, true),
                            Action::Marcher(lx), Action::Grimper(fb, lx), Action::Ecran(eb.id), Action::Marcher(lx + sf * p(self, 50.0)), Action::Ranger]);
                    }
                    x = bord + sf * p(self, 70.0);
                }
                TypeLien::Bas { x0, x1 } => {
                    // écran du dessous : il creuse, tombe, et le trou se referme derrière lui
                    let hx = clamp(x, x0 + p(self, 70.0), x1 - p(self, 70.0));
                    l_actions.extend([Action::Marcher(hx), Action::Creuser { x: hx, y: ea.sol, id: 0 },
                        Action::Tomber(eb.sol), Action::Ecran(eb.id), Action::Ranger, Action::Attendre(0.35, "land")]);
                    x = hx;
                }
                TypeLien::Haut { x0, x1 } => {
                    // écran du dessus : échelle, ascension, puis il remonte l'échelle avec lui
                    let hx = clamp(x, x0 + p(self, 70.0), x1 - p(self, 70.0));
                    l_actions.extend([Action::Marcher(hx), echelle(hx, eb.sol - p(self, 30.0), ea.sol, false),
                        Action::Grimper(eb.sol, hx), Action::Ecran(eb.id), Action::Marcher(hx + p(self, 50.0)), Action::Ranger]);
                    x = hx + p(self, 50.0);
                }
            }
        }
        l_actions.push(Action::Transit(false));
        self.acteurs[ai].file.extend(l_actions);
    }

    /// Pas de chemin à pied : il passe par un portail.
    fn voyage_portail(&mut self, ai: usize, cible: usize) {
        let e = self.ecran(cible).clone();
        let x = self.entre(e.gx, e.dx);
        let (d, d2) = (self.px(60.0), self.px(55.0));
        self.acteurs[ai].file.extend([Action::Transit(true), Action::OuvrirPortail(d2), Action::EntrerPortail { t: -1.0, x0: 0.0 },
            Action::FermerPortail, Action::Attendre(0.6, "idle"), Action::Teleporter { ecran: e.id, x: x - d / 2.0 }, Action::OuvrirPortail(0.0),
            Action::SortirPortail { dist: d, t: -1.0 }, Action::FermerPortail, Action::Transit(false)]);
    }

    /// Vide la file d'attente, sauf un passage d'écran ou une livraison en cours.
    fn interrompre(&mut self, ai: usize) -> bool {
        let a = &mut self.acteurs[ai];
        if a.occupe { return false; }
        if a.transit {
            if let Some(i) = a.file.iter().position(|x| matches!(x, Action::Transit(false))) { a.file.truncate(i + 1); }
        } else {
            a.file.clear();
            a.action = None;
        }
        true
    }

    pub fn demo_ecran(&mut self) {
        let ai = self.index(MOI).unwrap();
        self.interrompre(ai);
        let de = self.acteurs[ai].ecran;
        let ids: Vec<usize> = self.ecrans.iter().map(|e| e.id).filter(|&i| i != de).collect();
        if ids.is_empty() { return self.dire(MOI, "Il n'y a qu'un écran !", 3.0); }
        let suiv = *ids.iter().find(|&&i| i > de).unwrap_or(&ids[0]);
        self.aller_a(ai, suiv);
    }

    pub fn demo_portail(&mut self) {
        let ai = self.index(MOI).unwrap();
        self.interrompre(ai);
        let de = self.acteurs[ai].ecran;
        let n = self.ecrans.len();
        let cible = if n > 1 { self.ecrans[(self.ecrans.iter().position(|e| e.id == de).unwrap_or(0) + 1) % n].id } else { de };
        self.voyage_portail(ai, cible);
    }

    pub fn dire(&mut self, id: u64, texte: &str, duree: f32) {
        if let Some(a) = self.acteurs.iter_mut().find(|a| a.id == id) {
            a.bulle = Some(Bulle { titre: String::new(), texte: texte.into(), duree: Some(duree), cliquable: false });
        }
    }

    fn index(&self, id: u64) -> Option<usize> { self.acteurs.iter().position(|a| a.id == id) }

    /// Le personnage part livrer une lettre : il ouvre un portail, entre, l'envoi part, puis il revient.
    pub fn envoyer_lettre(&mut self, envoi: u64) {
        let ai = self.index(MOI).unwrap();
        self.interrompre(ai);
        let d = self.px(55.0);
        self.acteurs[ai].file.extend([
            Action::Occupe(true), Action::Lettre(true),
            Action::Bulle(Some(Bulle { titre: String::new(), texte: "Une lettre à livrer !".into(), duree: Some(2.0), cliquable: false })),
            Action::Attendre(1.2, "deliver"), Action::OuvrirPortail(d), Action::EntrerPortail { t: -1.0, x0: 0.0 },
            Action::LancerEnvoi(envoi), Action::Attendre(3.0, "idle"), Action::AttendreQue(Cond::Resultat, "idle"),
            Action::Lettre(false), Action::RouvrirPortail, Action::SortirPortail { dist: d, t: -1.0 }, Action::FermerPortail,
            Action::DireResultat, Action::Attendre(1.5, "wave"), Action::Occupe(false),
        ]);
    }

    pub fn resultat_envoi(&mut self, texte: String) { self.moi().resultat = Some(texte); }

    /// Un visiteur sort d'un portail pour apporter un message reçu.
    pub fn accueillir(&mut self, nom: &str, look: &str, titre: String, texte: String, msg: serde_json::Value, ecran: usize) {
        let e = self.ecran(ecran).clone();
        let n = self.acteurs.iter().filter(|a| a.visiteur).count() as f32;
        let x = clamp(self.entre(e.gx + self.px(80.0), e.dx - self.px(80.0)) + n * self.px(90.0), e.gx, e.dx);
        let id = self.nid();
        let mut v = Acteur::new(id, nom, look, x, e.sol, e.id);
        v.visiteur = true;
        v.visible = false;
        v.sens = if x < (e.gx + e.dx) / 2.0 { 1 } else { -1 };
        v.msg = Some(msg);
        v.lettre = true;
        let d = self.px(70.0);
        v.file.extend([
            Action::OuvrirPortail(0.0), Action::SortirPortail { dist: d, t: -1.0 }, Action::FermerPortail,
            Action::Bulle(Some(Bulle { titre, texte, duree: None, cliquable: true })), Action::AttendreQue(Cond::Lu, "deliver"),
        ]);
        v.file.extend(Self::depart_visiteur());
        self.acteurs.push(v);
    }

    fn depart_visiteur() -> Vec<Action> {
        vec![Action::Lettre(false), Action::Bulle(None), Action::Attendre(1.4, "wave"), Action::Sens(0),
            Action::OuvrirPortail(55.0), Action::EntrerPortail { t: -1.0, x0: 0.0 }, Action::FermerPortail, Action::Supprimer]
    }

    /// Clic sur un acteur (ou sa bulle).
    pub fn clic(&mut self, id: u64) {
        if id == MOI { self.sorties.push(Sortie::Menu(MOI)); return; }
        if let Some(a) = self.acteurs.iter_mut().find(|a| a.id == id) {
            if a.visiteur && !a.lu {
                a.lu = true;
                if let Some(m) = a.msg.clone() { self.sorties.push(Sortie::AccuserLecture(m)); }
            }
        }
    }

    // ---------- prise à la souris ----------
    /// Attrape le personnage (renvoie faux s'il est occupé à livrer une lettre, ou fantôme).
    pub fn saisir(&mut self, id: u64) -> bool {
        let Some(ai) = self.index(id) else { return false };
        let a = &self.acteurs[ai];
        if a.visiteur || a.fantome || a.occupe || !a.visible { return false; }
        if let Some(o) = self.acteurs[ai].dernier_objet.take() { self.ranger(o); }
        let a = &mut self.acteurs[ai];
        a.file.clear();
        a.action = None;
        a.transit = false;
        a.au_repos();
        a.porte = true;
        a.appui = Appui::Air;
        a.etoiles = 0.0;
        a.bulle = None;
        a.anim("dangle");
        true
    }

    /// Le curseur (x, y) tient le personnage par le col.
    pub fn tenir(&mut self, id: u64, x: f32, y: f32) {
        let col = self.px(66.0);
        let Some(a) = self.acteurs.iter_mut().find(|a| a.id == id && a.porte) else { return };
        if (x - a.x).abs() > 1.0 { a.sens = if x > a.x { 1 } else { -1 }; }
        a.x = x;
        a.y = y + col;
        a.anim("dangle");
    }

    /// Relâché avec la vitesse de la souris : il chute (et peut être lancé).
    pub fn lacher(&mut self, id: u64, vx: f32, vy: f32) {
        let max = self.px(1800.0);
        let Some(a) = self.acteurs.iter_mut().find(|a| a.id == id && a.porte) else { return };
        a.porte = false;
        a.file.push_back(Action::Chute { vx: vx.clamp(-max, max), vy: vy.clamp(-max, max), y0: a.y, para: false });
    }

    // ---------- soins ----------
    fn anim_repas(cle: &str) -> (&'static str, &'static str) {
        match cle { "pomme" => ("pomme", "eat_pomme"), "baie" => ("baie", "eat_baie"), _ => ("champi", "eat_champi") }
    }

    /// Prépare le personnage à une action de soin (il doit être vivant, posé, libre).
    fn disponible(&mut self) -> Result<usize, String> {
        if self.soins.mort { return Err("Il est mort… ressuscite-le d'abord.".into()); }
        let ai = self.index(MOI).ok_or("personnage introuvable")?;
        let a = &self.acteurs[ai];
        if a.porte { return Err("Pose-le d'abord !".into()); }
        if a.renverse || a.appui == Appui::Plafond || a.appui == Appui::Air { return Err("Il est en pleine acrobatie, réessaie dans un instant.".into()); }
        if !self.interrompre(ai) { return Err("Il livre une lettre, réessaie dans un instant.".into()); }
        Ok(ai)
    }

    pub fn nourrir(&mut self, cle: &str) -> Result<(), String> {
        if !self.soins.mort && self.soins.stock(cle) == 0 {
            let nom = crate::soins::ALIMENTS.iter().find(|a| a.0 == cle).map(|a| a.1).unwrap_or(cle);
            return Err(format!("Plus de {} en réserve.", nom.to_lowercase()));
        }
        let ai = self.disponible()?;
        let (cle, anim) = Self::anim_repas(cle);
        self.acteurs[ai].file.extend([Action::Nourrir(cle), Action::Attendre(2.6, anim),
            Action::Bulle(Some(Bulle { titre: String::new(), texte: "Miam !".into(), duree: Some(2.0), cliquable: false })), Action::Attendre(0.9, "happy")]);
        Ok(())
    }

    pub fn caresser(&mut self) -> Result<(), String> {
        let ai = self.disponible()?;
        self.acteurs[ai].file.extend([Action::Caresse, Action::Attendre(1.6, "happy")]);
        Ok(())
    }

    pub fn ressusciter(&mut self) -> Result<(), String> {
        if !self.soins.mort { return Err("Il est bien vivant !".into()); }
        self.soins.ressusciter();
        self.objets.retain(|o| o.t != TypeObjet::Tombe);
        let ai = self.index(MOI).ok_or("personnage introuvable")?;
        let a = &mut self.acteurs[ai];
        a.fantome = false;
        a.file.clear();
        a.action = None;
        a.au_repos();
        a.file.extend([Action::Occupe(true), Action::OuvrirPortail(0.0), Action::FermerPortail, Action::Caresse,
            Action::Bulle(Some(Bulle { titre: String::new(), texte: "Me revoilà ! Cette fois, nourris-moi…".into(), duree: Some(4.0), cliquable: false })),
            Action::Attendre(2.0, "happy"), Action::Occupe(false)]);
        Ok(())
    }

    /// Mort de faim : il s'effondre, une tombe apparaît, il devient fantôme.
    fn mourir(&mut self) {
        let Some(ai) = self.index(MOI) else { return };
        let a = &mut self.acteurs[ai];
        a.file.clear();
        a.action = None;
        a.occupe = false;
        a.transit = false;
        a.lettre = false;
        a.porte = false;
        a.renverse = false;
        if a.appui != Appui::Sol { a.file.push_back(Action::Chute { vx: 0.0, vy: 0.0, y0: a.y, para: false }); }
        a.file.extend([Action::Bulle(Some(Bulle { titre: String::new(), texte: "Je… n'en peux plus…".into(), duree: Some(2.5), cliquable: false })),
            Action::Attendre(1.5, "faible"), Action::Couche(true), Action::Attendre(2.5, "ko"), Action::Mourir]);
    }

    fn ranger(&mut self, id: u64) {
        if let Some(o) = self.objets.iter_mut().find(|o| o.id == id) { o.vie = o.vie.min(0.0); }
    }

    // ---------- surfaces : sol des écrans et dessus des fenêtres ----------
    /// Nouvelles plateformes (fenêtres) : ceux qui s'y tiennent suivent la fenêtre si elle bouge.
    pub fn definir_plateformes(&mut self, nouvelles: Vec<Plateforme>) {
        let anciennes = std::mem::replace(&mut self.plateformes, nouvelles);
        for a in &mut self.acteurs {
            let Appui::Fenetre(h) = a.appui else { continue };
            if a.porte { continue; }
            let (Some(o), Some(n)) = (anciennes.iter().find(|p| p.hwnd == h), self.plateformes.iter().find(|p| p.hwnd == h)) else { continue };
            a.x += n.gauche - o.gauche;
            a.y += n.y - o.y;
        }
    }

    fn soutenu(&self, a: &Acteur) -> bool {
        match a.appui {
            Appui::Fenetre(h) => {
                let marge = self.px(20.0);
                self.plateformes.iter().any(|p| p.hwnd == h && a.x >= p.x0 - marge && a.x <= p.x1 + marge && (a.y - p.y).abs() < 3.0)
            }
            _ => true,
        }
    }

    /// Première surface à la hauteur `y` ou en dessous, à l'abscisse `x`.
    fn surface_sous(&self, x: f32, y: f32) -> Option<(f32, Appui)> {
        let mut meilleur: Option<(f32, Appui)> = None;
        let mut garder = |sy: f32, ap: Appui| if sy >= y - 0.5 && meilleur.is_none_or(|m| sy < m.0) { meilleur = Some((sy, ap)); };
        for p in &self.plateformes { if x >= p.x0 && x <= p.x1 { garder(p.y, Appui::Fenetre(p.hwnd)); } }
        for e in &self.ecrans { if x >= e.x && x < e.r { garder(e.sol, Appui::Sol); } }
        meilleur
    }

    /// Écran qui contient le point, sinon le plus proche horizontalement.
    fn ecran_de(&self, x: f32, y: f32) -> usize {
        self.ecrans.iter().find(|e| x >= e.x && x < e.r && y > e.y && y <= e.b)
            .or_else(|| self.ecrans.iter().min_by(|a, b| {
                let d = |e: &Ecran| if x < e.x { e.x - x } else if x >= e.r { x - e.r } else { 0.0 } + if y < e.y { e.y - y } else if y > e.b { y - e.b } else { 0.0 };
                d(a).total_cmp(&d(b))
            }))
            .map(|e| e.id).unwrap_or(0)
    }

    /// Va sur le dessus d'une fenêtre : d'un saut si elle est basse, sinon par une échelle.
    fn aller_fenetre(&mut self, ai: usize) -> bool {
        let a = &self.acteurs[ai];
        let e = self.ecran(a.ecran).clone();
        let (ax, ay) = (a.x, a.y);
        let cands: Vec<Plateforme> = self.plateformes.iter().filter(|p| p.x1 - p.x0 > self.px(120.0) && p.y < ay - self.px(60.0)
            && p.y > e.y + self.px(110.0) && p.x0 >= e.x - 1.0 && p.x1 <= e.r + 1.0).cloned().collect();
        if cands.is_empty() { return false; }
        let p = cands[(self.hasard() * cands.len() as f32) as usize % cands.len()].clone();
        let h = p.hwnd;
        if ay - p.y <= self.px(190.0) {
            let tx = clamp(ax, p.x0 + self.px(30.0), p.x1 - self.px(30.0));
            let s = if ax <= tx { 1.0 } else { -1.0 };
            let depart = clamp(tx - s * self.px(80.0), e.gx, e.dx);
            self.acteurs[ai].file.extend([Action::Transit(true), Action::Marcher(depart),
                Action::Sauter { x: tx, y: p.y, t: -1.0, x0: 0.0, y0: 0.0 }, Action::Appui(Appui::Fenetre(h)), Action::Attendre(0.3, "land"), Action::Transit(false)]);
            return true;
        }
        let gauche = (ax - p.x0).abs() <= (ax - p.x1).abs();
        for cote in [gauche, !gauche] {
            let (s, lx) = if cote { (1.0, p.x0 - self.px(14.0)) } else { (-1.0, p.x1 + self.px(14.0)) };
            let pied = lx - s * self.px(45.0);
            if lx < e.x + self.px(20.0) || lx > e.r - self.px(20.0) || pied < e.x || pied > e.r { continue; }
            let (haut, pas) = (p.y - self.px(30.0), lx + s * self.px(50.0));
            self.acteurs[ai].file.extend([Action::Transit(true), Action::Marcher(pied), Action::Sens(s as i32),
                Action::Echelle { x: lx, y_haut: haut, y_bas: ay, vers_bas: false, id: 0 },
                Action::Marcher(lx), Action::Grimper(p.y, lx), Action::Appui(Appui::Fenetre(h)), Action::Marcher(pas),
                Action::Ranger, Action::Transit(false)]);
            return true;
        }
        false
    }

    /// Quitte le dessus d'une fenêtre : petit saut s'il peut se rattraper, sinon échelle vers le bas.
    fn descendre_fenetre(&mut self, ai: usize, p: &Plateforme) {
        let a = &self.acteurs[ai];
        let s = if a.x - p.x0 < p.x1 - a.x { -1.0 } else { 1.0 };
        let bord = if s < 0.0 { p.x0 + self.px(8.0) } else { p.x1 - self.px(8.0) };
        let lx = bord + s * self.px(26.0);
        let dessous = self.surface_sous(lx, p.y + 1.0).map(|d| d.0).unwrap_or(p.y);
        let chute = dessous - p.y;
        let sauter = chute <= self.px(170.0) || (chute <= self.px(600.0) && self.soins.parachutes >= 2);
        let (vx, vy, haut, pas) = (s * self.px(110.0), -self.px(320.0), p.y - self.px(30.0), lx + s * self.px(40.0));
        let f = &mut self.acteurs[ai].file;
        f.extend([Action::Transit(true), Action::Marcher(bord), Action::Sens(s as i32)]);
        if sauter {
            f.push_back(Action::Chute { vx, vy, y0: p.y, para: false });
        } else {
            f.extend([Action::Echelle { x: lx, y_haut: haut, y_bas: dessous, vers_bas: true, id: 0 },
                Action::Marcher(lx), Action::Grimper(dessous, lx), Action::Appui(Appui::Sol), Action::Marcher(pas), Action::Ranger]);
        }
        f.push_back(Action::Transit(false));
    }

    /// Escalade un bord d'écran (là où il n'y a pas d'écran voisin), puis marche au plafond ou redescend.
    fn escalader(&mut self, ai: usize) -> bool {
        let e = self.ecran(self.acteurs[ai].ecran).clone();
        let libres: Vec<i32> = [-1, 1].into_iter()
            .filter(|&s| !self.liens.iter().any(|l| l.de == e.id && matches!(l.t, TypeLien::Cote { sens, .. } if sens == s))).collect();
        if libres.is_empty() { return false; }
        let s = libres[(self.hasard() * libres.len() as f32) as usize % libres.len()];
        let wx = if s < 0 { e.x + self.px(16.0) } else { e.r - self.px(16.0) };
        let haut = e.y + self.px(108.0);
        if haut >= e.sol - self.px(150.0) { return false; }
        let plafond = self.hasard() < 0.7;
        let x2 = self.entre(e.gx, e.dx);
        let pause = self.entre(1.0, 3.0);
        let lacher = self.soins.parachutes >= 2 && self.hasard() < 0.35;
        let f = &mut self.acteurs[ai].file;
        f.extend([Action::Transit(true), Action::Marcher(wx), Action::Sens(s), Action::Grimper(haut, wx)]);
        if plafond {
            f.extend([Action::Renverser(true), Action::Marcher(x2), Action::Attendre(pause, "idle")]);
            if lacher {
                // il se laisse tomber… et ouvre son parachute
                f.extend([Action::Renverser(false), Action::Chute { vx: 0.0, vy: 0.0, y0: 0.0, para: false }]);
            } else {
                f.extend([Action::Marcher(wx), Action::Renverser(false), Action::Sens(s), Action::Grimper(e.sol, wx), Action::Appui(Appui::Sol)]);
            }
        } else {
            f.extend([Action::Attendre(0.8, "climb"), Action::Grimper(e.sol, wx), Action::Appui(Appui::Sol)]);
        }
        f.push_back(Action::Transit(false));
        true
    }

    // ---------- promenade ----------
    fn flaner(&mut self, ai: usize) {
        let a = &self.acteurs[ai];
        if a.id == MOI && self.soins.mort {
            // fantôme : il flotte près de sa tombe
            if self.hasard() < 0.08 {
                self.dire(MOI, "Clique-moi pour me ressusciter…", 4.0);
            }
            self.acteurs[ai].file.push_back(Action::Attendre(3.0, "idle"));
            return;
        }
        if let Appui::Fenetre(h) = a.appui {
            let Some(p) = self.plateformes.iter().find(|p| p.hwnd == h && a.x >= p.x0 - self.px(20.0) && a.x <= p.x1 + self.px(20.0)).cloned() else {
                self.acteurs[ai].file.push_back(Action::Attendre(0.5, "idle"));
                return;
            };
            let r = self.hasard();
            if r < 0.25 { return self.descendre_fenetre(ai, &p); }
            if r < 0.65 {
                let x = self.entre(p.x0 + self.px(20.0), p.x1 - self.px(20.0));
                self.acteurs[ai].file.push_back(Action::Marcher(x));
            } else if r < 0.82 {
                let d = self.entre(4.0, 9.0);
                self.acteurs[ai].file.push_back(Action::Attendre(d, "sit"));
            } else {
                let d = self.entre(1.5, 4.0);
                self.acteurs[ai].file.push_back(Action::Attendre(d, "idle"));
            }
            return;
        }
        let e = self.ecran(a.ecran).clone();
        // tous les autres écrans : à pied si un chemin existe, sinon par portail
        let autres: Vec<usize> = self.ecrans.iter().map(|x| x.id).filter(|&i| i != e.id).collect();
        let affame = a.id == MOI && self.soins.satiete < 25.0;
        let mut r = self.hasard();
        if affame && r < 0.35 {
            let d = self.entre(3.0, 6.0);
            self.acteurs[ai].file.push_back(Action::Attendre(d, "faible"));
            return;
        }
        if !autres.is_empty() && r < 0.16 {
            let c = autres[(self.hasard() * autres.len() as f32) as usize % autres.len()];
            self.aller_a(ai, c);
            let ec = self.ecran(c).clone();
            let x = self.entre(ec.gx, ec.dx);
            self.acteurs[ai].file.push_back(Action::Marcher(x));
            return;
        }
        if r < 0.26 && !affame && self.aller_fenetre(ai) { return; }
        if r < 0.33 && !affame && self.escalader(ai) { return; }
        if r < 0.33 { r = 0.4; }
        if r < 0.62 {
            let x = self.entre(e.gx, e.dx);
            self.acteurs[ai].file.push_back(Action::Marcher(x));
        } else if r < 0.74 {
            let d = self.entre(4.0, 9.0);
            self.acteurs[ai].file.push_back(Action::Attendre(d, "sit"));
        } else if r < 0.79 {
            self.acteurs[ai].file.push_back(Action::Attendre(1.5, "wave"));
        } else {
            let d = self.entre(1.5, 4.0);
            let s = if self.hasard() < 0.5 { 1 } else { -1 };
            self.acteurs[ai].file.extend([Action::Sens(s), Action::Attendre(d, "idle")]);
        }
    }

    // ---------- boucle ----------
    pub fn maj(&mut self, dt: f32) {
        self.temps += dt;
        for o in &mut self.objets {
            if o.t == TypeObjet::Portail {
                let v = dt * 2.2;
                o.p = if o.cible > o.p { (o.p + v).min(o.cible) } else { (o.p - v).max(o.cible) };
            } else if o.t != TypeObjet::Tombe {
                o.vie -= dt;
            }
        }
        self.objets.retain(|o| if o.t == TypeObjet::Portail { !(o.cible == 0.0 && o.p <= 0.0) } else { o.vie > -2.0 });

        // Tamagotchi
        if self.index(MOI).is_some() {
            let h = self.hasard();
            for ev in self.soins.maj(dt, self.vitesse_soins, h) {
                match ev {
                    Evenement::Recolte(c) => {
                        let nom = crate::soins::ALIMENTS.iter().find(|a| a.0 == c).map(|a| a.1).unwrap_or(c);
                        self.dire(MOI, &format!("Le jardin a donné : {} !", nom.to_lowercase()), 3.5);
                    }
                    Evenement::Parachute => self.dire(MOI, "Nouveau parachute plié !", 3.0),
                    Evenement::Faim => self.dire(MOI, "J'ai faim… (clique sur moi)", 5.0),
                    Evenement::Mort => self.mourir(),
                }
            }
            self.rappel_faim += dt;
            if self.rappel_faim > 240.0 {
                self.rappel_faim = 0.0;
                if !self.soins.mort && self.soins.satiete < 25.0 { self.dire(MOI, "J'ai faim…", 4.0); }
            }
        }

        for ai in 0..self.acteurs.len() {
            {
                let a = &mut self.acteurs[ai];
                a.anim_t += dt;
                a.etoiles = (a.etoiles - dt).max(0.0);
                a.coeurs = (a.coeurs - dt).max(0.0);
                if let Some(b) = &mut a.bulle { if let Some(d) = &mut b.duree { *d -= dt; if *d <= 0.0 { a.bulle = None; } } }
                if a.porte { continue; }
            }
            // la fenêtre sous ses pieds a disparu, bougé ou été recouverte : il tombe
            let a = &self.acteurs[ai];
            let en_mouvement = matches!(a.action, Some(Action::Grimper(..) | Action::Chute { .. } | Action::Sauter { .. }));
            if !en_mouvement && !self.soutenu(a) {
                let a = &mut self.acteurs[ai];
                a.action = None;
                a.file.push_front(Action::Chute { vx: 0.0, vy: 0.0, y0: a.y, para: false });
            }
            if self.acteurs[ai].action.is_none() {
                if self.acteurs[ai].file.is_empty() && !self.acteurs[ai].visiteur { self.flaner(ai); }
                let a = &mut self.acteurs[ai];
                a.action = a.file.pop_front();
                a.neuf = true;
            }
            if let Some(mut act) = self.acteurs[ai].action.take() {
                let fini = self.pas(ai, &mut act, dt);
                self.acteurs[ai].neuf = false;
                if !fini && self.acteurs[ai].action.is_none() { self.acteurs[ai].action = Some(act); }
            }
        }
        self.acteurs.retain(|a| !a.supprimer);
    }
    fn portail_de(&self, ai: usize) -> Option<&Objet> {
        let id = self.acteurs[ai].portail?;
        self.objets.iter().find(|o| o.id == id)
    }

    /// Fait avancer une action ; renvoie vrai quand elle est terminée.
    fn pas(&mut self, ai: usize, act: &mut Action, dt: f32) -> bool {
        let neuf = self.acteurs[ai].neuf;
        let (vitesse, grimpe, gravite) = (self.px(75.0), self.px(70.0), self.px(2400.0));
        match act {
            Action::Marcher(x) => {
                // plus lent au plafond, et quand il meurt de faim
                let lent = if self.acteurs[ai].id == MOI && self.soins.satiete <= 0.0 { 0.6 } else { 1.0 };
                let a = &mut self.acteurs[ai];
                let v = vitesse * lent * if a.renverse { 0.6 } else { 1.0 };
                let m = a.marche();
                a.anim(m);
                let d = *x - a.x;
                if d.abs() < 1.5 { a.x = *x; return true; }
                a.sens = d.signum() as i32;
                a.x += d.signum() * d.abs().min(v * dt);
                false
            }
            Action::Chute { vx, vy, y0, para } => {
                let (plafond_para, glisse) = (self.px(85.0), self.px(20.0));
                if neuf {
                    let a = &mut self.acteurs[ai];
                    a.appui = Appui::Air;
                    a.renverse = false;
                    a.couche = false;
                    if *y0 == 0.0 { *y0 = a.y; }
                }
                let y_avant = self.acteurs[ai].y;
                if *para {
                    *vy = (*vy + gravite * dt).min(plafond_para);
                    *vx *= 1.0 - (2.0 * dt).min(1.0);
                    let a = &mut self.acteurs[ai];
                    a.anim("parachute");
                    a.x += (*vx + (self.temps * 2.0).sin() * glisse) * dt;
                } else {
                    *vy += gravite * dt;
                    let a = &mut self.acteurs[ai];
                    a.anim("fall");
                    a.x += *vx * dt;
                }
                self.acteurs[ai].y += *vy * dt;
                *y0 = y0.min(self.acteurs[ai].y);
                // rebond sur les bords extérieurs des écrans
                let (x, y) = (self.acteurs[ai].x, self.acteurs[ai].y);
                if !self.ecrans.iter().any(|e| x >= e.x && x < e.r) {
                    let e = self.ecran(self.ecran_de(x, y)).clone();
                    self.acteurs[ai].x = clamp(x, e.x + self.px(20.0), e.r - self.px(20.0));
                    *vx = -*vx * 0.4;
                }
                let x = self.acteurs[ai].x;
                // ouverture du parachute si la chute s'annonce longue
                if !*para && *vy > 0.0 && y - *y0 > self.px(140.0) && self.acteurs[ai].id == MOI {
                    let sol = self.surface_sous(x, y).map(|s| s.0).unwrap_or(y);
                    if sol - y > self.px(220.0) && self.soins.prendre_parachute() {
                        *para = true;
                        *vy = vy.min(self.px(300.0));
                        self.acteurs[ai].parachute = true;
                    }
                }
                // atterrissage sur la première surface traversée
                if *vy > 0.0 {
                    if let Some((sy, appui)) = self.surface_sous(x, y_avant).filter(|s| s.0 <= y + 0.5) {
                        let hauteur = sy - *y0;
                        let ecran = self.ecran_de(x, sy - 1.0);
                        let (crash, leger) = (self.px(380.0), self.px(30.0));
                        let a = &mut self.acteurs[ai];
                        a.y = sy;
                        a.appui = appui;
                        a.ecran = ecran;
                        let avait_para = a.parachute;
                        a.parachute = false;
                        let suite: Vec<Action> = if avait_para {
                            vec![Action::Attendre(0.4, "land")]
                        } else if hauteur > crash {
                            a.etoiles = 3.5;
                            vec![Action::Couche(true), Action::Attendre(2.2, "ko"), Action::Couche(false), Action::Attendre(1.2, "faible"),
                                Action::Bulle(Some(Bulle { titre: String::new(), texte: "Aïe… sans parachute, ça pique.".into(), duree: Some(3.0), cliquable: false }))]
                        } else if hauteur > leger {
                            vec![Action::Attendre(0.35, "land")]
                        } else {
                            vec![]
                        };
                        if hauteur > crash && !avait_para && a.id == MOI { self.soins.ecrasement(); }
                        let a = &mut self.acteurs[ai];
                        for s in suite.into_iter().rev() { a.file.push_front(s); }
                        return true;
                    }
                }
                false
            }
            Action::Appui(ap) => {
                let (x, y) = (self.acteurs[ai].x, self.acteurs[ai].y);
                if *ap == Appui::Sol { self.acteurs[ai].ecran = self.ecran_de(x, y - 1.0); }
                self.acteurs[ai].appui = *ap;
                true
            }
            Action::Renverser(v) => {
                let e = self.ecran(self.acteurs[ai].ecran).clone();
                let haut = e.y + self.px(108.0);
                let a = &mut self.acteurs[ai];
                a.renverse = *v;
                if *v { a.y = e.y; a.appui = Appui::Plafond; a.sens = -a.sens; } else { a.y = haut; a.appui = Appui::Air; }
                true
            }
            Action::Couche(v) => { self.acteurs[ai].couche = *v; true }
            Action::Ranger => {
                if let Some(o) = self.acteurs[ai].dernier_objet.take() { self.ranger(o); }
                true
            }
            Action::Nourrir(cle) => {
                if self.soins.nourrir(cle) { self.acteurs[ai].coeurs = 2.0; }
                true
            }
            Action::Caresse => {
                self.soins.caresser();
                self.acteurs[ai].coeurs = 2.5;
                true
            }
            Action::Mourir => {
                let (x, y) = (self.acteurs[ai].x, self.acteurs[ai].y);
                let id = self.nid();
                self.objets.push(Objet { id, t: TypeObjet::Tombe, x, y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 1.0, cible: 1.0, vie: f32::MAX });
                let a = &mut self.acteurs[ai];
                a.couche = false;
                a.fantome = true;
                a.x += self.k * 3.0 * 12.0;
                a.bulle = Some(Bulle { titre: String::new(), texte: "Adieu… (clique-moi pour me ressusciter)".into(), duree: Some(8.0), cliquable: false });
                true
            }
            Action::Attendre(reste, anim) => {
                self.acteurs[ai].anim(anim);
                *reste -= dt;
                *reste <= 0.0
            }
            Action::AttendreQue(c, anim) => {
                self.acteurs[ai].anim(anim);
                match c {
                    Cond::Lu => self.acteurs[ai].lu,
                    Cond::Resultat => self.acteurs[ai].resultat.is_some(),
                }
            }
            Action::Grimper(y, x) => {
                let a = &mut self.acteurs[ai];
                a.anim("climb");
                a.x = *x;
                let d = *y - a.y;
                if d.abs() < 1.5 { a.y = *y; return true; }
                a.y += d.signum() * d.abs().min(grimpe * dt);
                false
            }
            Action::Tomber(y) => {
                let a = &mut self.acteurs[ai];
                if neuf { a.vy = 0.0; }
                a.anim("fall");
                a.vy += gravite * dt;
                a.y += a.vy * dt;
                if a.y >= *y { a.y = *y; return true; }
                false
            }
            Action::Sauter { x, y, t, x0, y0 } => {
                let a = &mut self.acteurs[ai];
                if *t < 0.0 { *t = 0.0; *x0 = a.x; *y0 = a.y; a.anim("jump"); if *x != a.x { a.sens = (*x - a.x).signum() as i32; } }
                *t += dt;
                let k = (*t / 0.7).min(1.0);
                a.x = *x0 + (*x - *x0) * k;
                a.y = *y0 + (*y - *y0) * k - self.k * 90.0 * 4.0 * k * (1.0 - k);
                k >= 1.0
            }
            Action::Echelle { x, y_haut, y_bas, vers_bas, id } => {
                if neuf {
                    *id = self.nid();
                    self.objets.push(Objet { id: *id, t: TypeObjet::Echelle, x: *x, y: 0.0, y_haut: *y_haut, y_bas: *y_bas, vers_bas: *vers_bas, p: 0.0, cible: 1.0, vie: 90.0 });
                    self.acteurs[ai].anim("build");
                    self.acteurs[ai].dernier_objet = Some(*id);
                }
                let duree = (2.0 * (*y_bas - *y_haut) / self.px(400.0)).clamp(1.5, 5.0);
                let Some(o) = self.objets.iter_mut().find(|o| o.id == *id) else { return true; };
                o.p += dt / duree;
                o.p >= 1.0
            }
            Action::Creuser { x, y, id } => {
                if neuf {
                    *id = self.nid();
                    self.objets.push(Objet { id: *id, t: TypeObjet::Trou, x: *x, y: *y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 0.0, cible: 1.0, vie: 90.0 });
                    self.acteurs[ai].anim("dig");
                    self.acteurs[ai].dernier_objet = Some(*id);
                }
                let Some(o) = self.objets.iter_mut().find(|o| o.id == *id) else { return true; };
                o.p += dt / 2.8;
                if o.p >= 1.0 { self.acteurs[ai].x = *x; return true; }
                false
            }
            Action::OuvrirPortail(dx) => {
                if neuf {
                    let id = self.nid();
                    let a = &self.acteurs[ai];
                    let (x, y) = (a.x + a.sens as f32 * *dx, a.y);
                    self.objets.push(Objet { id, t: TypeObjet::Portail, x, y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 0.0, cible: 1.0, vie: 999.0 });
                    let a = &mut self.acteurs[ai];
                    a.portail = Some(id);
                    if a.visible { a.anim("portal"); }
                }
                self.portail_de(ai).map(|o| o.p >= 1.0).unwrap_or(true)
            }
            Action::RouvrirPortail => {
                let id = self.acteurs[ai].portail;
                match self.objets.iter_mut().find(|o| Some(o.id) == id) {
                    Some(o) => { o.cible = 1.0; o.p >= 1.0 }
                    None => {
                        // le portail s'est refermé entre-temps : on en rouvre un
                        *act = Action::OuvrirPortail(0.0);
                        self.acteurs[ai].neuf = true;
                        let mut a2 = Action::OuvrirPortail(0.0);
                        let r = self.pas(ai, &mut a2, dt);
                        *act = a2;
                        r
                    }
                }
            }
            Action::EntrerPortail { t, x0 } => {
                let px_ = self.portail_de(ai).map(|o| o.x);
                let a = &mut self.acteurs[ai];
                if *t < 0.0 { *t = 0.0; *x0 = a.x; }
                let m = a.marche();
                a.anim(m);
                *t += dt;
                let k = (*t / 0.8).min(1.0);
                if let Some(px_) = px_ { a.x = *x0 + (px_ - *x0) * k; }
                a.echelle = 1.0 - k;
                if k >= 1.0 { a.visible = false; a.echelle = 1.0; return true; }
                false
            }
            Action::SortirPortail { dist, t } => {
                let px_ = self.portail_de(ai).map(|o| o.x);
                let a = &mut self.acteurs[ai];
                if *t < 0.0 || neuf { *t = 0.0; if let Some(p) = px_ { a.x = p; } a.visible = true; a.echelle = 0.0; }
                let m = a.marche();
                a.anim(m);
                let x_avant = a.x;
                *t += dt;
                let k = (*t / 0.8).min(1.0);
                if let Some(p) = px_ { a.x = p + a.sens as f32 * *dist * k; } else { a.x = x_avant; }
                a.echelle = k;
                k >= 1.0
            }
            Action::FermerPortail => {
                let id = self.acteurs[ai].portail;
                if let Some(o) = self.objets.iter_mut().find(|o| Some(o.id) == id) { o.cible = 0.0; }
                true
            }
            Action::Ecran(e) => { self.acteurs[ai].ecran = *e; true }
            Action::PoserY(y) => { self.acteurs[ai].y = *y; true }
            Action::Sens(s) => { let a = &mut self.acteurs[ai]; a.sens = if *s == 0 { -a.sens } else { *s }; true }
            Action::Lettre(v) => { self.acteurs[ai].lettre = *v; true }
            Action::Transit(v) => { self.acteurs[ai].transit = *v; true }
            Action::Occupe(v) => { self.acteurs[ai].occupe = *v; true }
            Action::Bulle(b) => { self.acteurs[ai].bulle = b.take(); true }
            Action::Teleporter { ecran, x } => {
                let sol = self.ecran(*ecran).sol;
                let a = &mut self.acteurs[ai];
                a.ecran = *ecran; a.x = *x; a.y = sol;
                true
            }
            Action::LancerEnvoi(n) => { self.sorties.push(Sortie::Envoyer(*n)); true }
            Action::DireResultat => {
                let a = &mut self.acteurs[ai];
                if let Some(r) = a.resultat.take() { a.bulle = Some(Bulle { titre: String::new(), texte: r, duree: Some(6.0), cliquable: false }); }
                true
            }
            Action::Supprimer => { self.acteurs[ai].supprimer = true; true }
        }
    }
}

/// Liens entre écrans qui se touchent (tolérance `tol` pixels), comme dans « Paramètres > Affichage ».
pub fn calculer_liens(e: &[Ecran], tol: f32) -> Vec<Lien> {
    let mut l = vec![];
    for a in e {
        for b in e {
            if a.id == b.id { continue; }
            let ch_y = a.b.min(b.b) - a.y.max(b.y);
            let (x0, x1) = (a.x.max(b.x), a.r.min(b.r));
            if (a.r - b.x).abs() <= tol && ch_y > 40.0 { l.push(Lien { de: a.id, vers: b.id, t: TypeLien::Cote { sens: 1, x: a.r } }); }
            if (b.r - a.x).abs() <= tol && ch_y > 40.0 { l.push(Lien { de: a.id, vers: b.id, t: TypeLien::Cote { sens: -1, x: a.x } }); }
            if (a.b - b.y).abs() <= tol && x1 - x0 > 160.0 { l.push(Lien { de: a.id, vers: b.id, t: TypeLien::Bas { x0, x1 } }); }
            if (b.b - a.y).abs() <= tol && x1 - x0 > 160.0 { l.push(Lien { de: a.id, vers: b.id, t: TypeLien::Haut { x0, x1 } }); }
        }
    }
    l
}

/// Description lisible des liens (pour le panneau).
pub fn decrire_liens(ecrans: &[Ecran], liens: &[Lien]) -> Vec<String> {
    let num = |id: usize| ecrans.iter().position(|e| e.id == id).map(|i| i + 1).unwrap_or(0);
    let mut v = vec![];
    for l in liens {
        let (a, b) = (num(l.de), num(l.vers));
        let (ea, eb) = (&ecrans[a - 1], &ecrans[b - 1]);
        let s = match l.t {
            TypeLien::Cote { sens, .. } => {
                let cote = if sens > 0 { "à droite" } else { "à gauche" };
                let d = eb.sol - ea.sol;
                let comment = if d.abs() <= 8.0 { "il passe à pied".to_string() }
                    else if d < 0.0 { format!("plus haut de {} px : échelle", -d as i32) }
                    else if d <= 160.0 { format!("plus bas de {} px : il saute", d as i32) }
                    else { format!("plus bas de {} px : échelle", d as i32) };
                format!("Écran {b} {cote} de l'écran {a} ({comment})")
            }
            TypeLien::Bas { .. } => format!("Écran {b} sous l'écran {a} : il creuse un trou"),
            TypeLien::Haut { .. } => format!("Écran {b} au-dessus de l'écran {a} : il pose une échelle"),
        };
        v.push(s);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ecran(id: usize, x: f32, y: f32, w: f32, h: f32) -> Ecran {
        Ecran { id, nom: format!("E{id}"), x, y, r: x + w, b: y + h, sol: y + h - 48.0, gx: x, dx: x + w, principal: id == 0 }
    }

    /// Personnage visible au milieu de l'écran principal, sans entrée par portail ni promenade en cours.
    fn pret(ecrans: Vec<Ecran>) -> Monde {
        let mut m = Monde::new("Test", "aventurier");
        m.definir_ecrans(ecrans, 1.0);
        m.objets.clear();
        let e = m.ecran(0).clone();
        let a = m.moi();
        a.file.clear();
        a.action = None;
        a.visible = true;
        a.occupe = false;
        a.x = (e.gx + e.dx) / 2.0;
        m
    }

    /// Fait voyager le personnage de l'écran 0 à l'écran `cible` et renvoie les animations vues.
    fn voyage(ecrans: Vec<Ecran>, cible: usize) -> (Monde, Vec<&'static str>) {
        let mut m = pret(ecrans);
        let ai = m.index(MOI).unwrap();
        m.aller_a(ai, cible);
        let mut anims = vec![];
        for _ in 0..4000 {
            m.maj(0.05);
            let a = &m.acteurs[ai];
            if anims.last() != Some(&a.anim) { anims.push(a.anim); }
            if !a.transit && a.ecran == cible && a.file.len() <= 1 { break; }
        }
        (m, anims)
    }

    fn arrive(m: &Monde, cible: usize) {
        let a = &m.acteurs[0];
        let e = m.ecran(cible);
        assert_eq!(a.ecran, cible);
        assert!((a.y - e.sol).abs() < 2.0, "y = {} au lieu du sol {}", a.y, e.sol);
        assert!(a.x >= e.x && a.x <= e.r, "x = {} hors de l'écran {}..{}", a.x, e.x, e.r);
    }

    #[test]
    fn cote_a_cote_meme_hauteur() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 1920.0, 0.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(!anims.contains(&"climb"), "{anims:?}");
    }

    #[test]
    fn cote_a_cote_plus_haut_echelle() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 1920.0, -400.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(anims.contains(&"build") && anims.contains(&"climb"), "{anims:?}");
    }

    #[test]
    fn cote_a_cote_un_peu_plus_bas_saut() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, -1920.0, 100.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(anims.contains(&"jump"), "{anims:?}");
    }

    #[test]
    fn cote_a_cote_beaucoup_plus_bas_echelle() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 1920.0, 500.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(anims.contains(&"climb"), "{anims:?}");
    }

    #[test]
    fn ecran_dessous_trou() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 200.0, 1080.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(anims.contains(&"dig") && anims.contains(&"fall"), "{anims:?}");
    }

    #[test]
    fn ecran_dessus_echelle_rangee_puis_trou() {
        let ecrans = vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 0.0, -1080.0, 1920.0, 1080.0)];
        let (mut m, anims) = voyage(ecrans, 1);
        arrive(&m, 1);
        assert!(anims.contains(&"build") && anims.contains(&"climb"), "{anims:?}");
        m.acteurs[0].file.push_back(Action::Attendre(10.0, "idle")); // pas de promenade pendant la vérification
        for _ in 0..60 { m.maj(0.05); }
        // une fois en haut, il a rangé son échelle
        assert!(!m.objets.iter().any(|o| o.t == TypeObjet::Echelle), "l'échelle doit disparaître");
        // retour : plus d'échelle, il creuse pour redescendre
        let ai = 0;
        m.interrompre(ai);
        m.aller_a(ai, 0);
        let mut vu = vec![];
        for _ in 0..4000 {
            m.maj(0.05);
            let a = &m.acteurs[ai];
            if vu.last() != Some(&a.anim) { vu.push(a.anim); }
            if !a.transit && a.ecran == 0 { break; }
        }
        arrive(&m, 0);
        assert!(vu.contains(&"dig") && vu.contains(&"fall"), "{vu:?}");
    }

    #[test]
    fn ecrans_separes_portail() {
        let (m, anims) = voyage(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 3000.0, 0.0, 1920.0, 1080.0)], 1);
        arrive(&m, 1);
        assert!(anims.contains(&"portal"), "{anims:?}");
    }

    #[test]
    fn livraison_et_visiteur() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        m.envoyer_lettre(1);
        let mut envoye = false;
        for _ in 0..400 {
            m.maj(0.05);
            for s in std::mem::take(&mut m.sorties) { if let Sortie::Envoyer(1) = s { envoye = true; m.resultat_envoi("Livré".into()); } }
        }
        assert!(envoye);
        assert!(m.acteurs[0].visible && !m.acteurs[0].occupe);
        m.accueillir("Léa", "bleu", "✉ Léa".into(), "Salut".into(), serde_json::json!({}), 0);
        for _ in 0..100 { m.maj(0.05); }
        let v = m.acteurs.iter().find(|a| a.visiteur).unwrap();
        assert!(v.visible && v.anim == "deliver" && v.bulle.is_some());
        let id = v.id;
        m.clic(id);
        for _ in 0..200 { m.maj(0.05); }
        assert!(m.acteurs.iter().all(|a| !a.visiteur), "le visiteur doit repartir");
    }

    /// Lâche le personnage en (x, y) et le laisse retomber ; renvoie les animations vues.
    fn lacher_de(m: &mut Monde, x: f32, y: f32) -> Vec<&'static str> {
        assert!(m.saisir(MOI));
        m.tenir(MOI, x, y);
        m.lacher(MOI, 0.0, 0.0);
        let mut vu = vec![];
        for _ in 0..600 {
            m.maj(0.05);
            let a = &m.acteurs[0];
            if vu.last() != Some(&a.anim) { vu.push(a.anim); }
            if a.appui != Appui::Air && a.action.is_none() && a.file.is_empty() { break; }
        }
        vu
    }

    #[test]
    fn lache_de_haut_avec_parachute() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        let (para, sante) = (m.soins.parachutes, m.soins.sante);
        let vu = lacher_de(&mut m, 900.0, 100.0);
        assert!(vu.contains(&"parachute"), "{vu:?}");
        assert_eq!(m.soins.parachutes, para - 1);
        assert_eq!(m.soins.sante, sante, "pas de blessure avec un parachute");
        assert!((m.acteurs[0].y - m.ecran(0).sol).abs() < 1.0);
    }

    #[test]
    fn lache_sans_parachute_il_s_ecrase() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        m.soins.parachutes = 0;
        let sante = m.soins.sante;
        let vu = lacher_de(&mut m, 900.0, 100.0);
        assert!(vu.contains(&"fall") && vu.contains(&"ko"), "{vu:?}");
        assert!(m.soins.sante < sante);
    }

    #[test]
    fn atterrit_sur_une_fenetre_puis_tombe_quand_elle_disparait() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        m.soins.parachutes = 0;
        m.definir_plateformes(vec![Plateforme { hwnd: 42, x0: 600.0, x1: 1200.0, y: 700.0, gauche: 600.0 }]);
        lacher_de(&mut m, 900.0, 500.0);
        assert_eq!(m.acteurs[0].appui, Appui::Fenetre(42));
        assert!((m.acteurs[0].y - 700.0).abs() < 1.0);
        // la fenêtre monte de 50 px : il suit
        m.definir_plateformes(vec![Plateforme { hwnd: 42, x0: 600.0, x1: 1200.0, y: 650.0, gauche: 600.0 }]);
        assert!((m.acteurs[0].y - 650.0).abs() < 1.0);
        // elle est fermée : il retombe au sol
        m.definir_plateformes(vec![]);
        for _ in 0..200 { m.maj(0.05); if m.acteurs[0].appui == Appui::Sol { break; } }
        assert_eq!(m.acteurs[0].appui, Appui::Sol);
        assert!((m.acteurs[0].y - m.ecran(0).sol).abs() < 1.0);
    }

    #[test]
    fn escalade_un_bord_puis_revient_au_sol() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        assert!(m.escalader(0));
        let mut vu = vec![];
        let mut plafond = false;
        for _ in 0..4000 {
            m.maj(0.05);
            let a = &m.acteurs[0];
            if vu.last() != Some(&a.anim) { vu.push(a.anim); }
            plafond |= a.renverse;
            if !a.transit { break; }
        }
        assert!(vu.contains(&"climb"), "{vu:?}");
        let a = &m.acteurs[0];
        assert!(!a.renverse && a.appui == Appui::Sol && (a.y - m.ecran(0).sol).abs() < 1.0, "plafond vu : {plafond}, {vu:?}");
    }

    #[test]
    fn repas_mort_et_resurrection() {
        let mut m = pret(vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0)]);
        m.soins.satiete = 10.0;
        let pommes = m.soins.pomme;
        m.nourrir("pomme").unwrap();
        let mut vu = vec![];
        for _ in 0..80 { m.maj(0.05); vu.push(m.acteurs[0].anim); }
        assert!(vu.contains(&"eat_pomme"));
        assert_eq!(m.soins.pomme, pommes - 1);
        assert!(m.soins.satiete > 35.0);
        // famine accélérée jusqu'à la mort
        m.vitesse_soins = 5000.0;
        for _ in 0..400 { m.maj(0.05); if m.acteurs[0].fantome { break; } }
        assert!(m.soins.mort && m.acteurs[0].fantome);
        assert!(m.objets.iter().any(|o| o.t == TypeObjet::Tombe));
        assert!(m.nourrir("pomme").is_err());
        m.vitesse_soins = 1.0;
        m.ressusciter().unwrap();
        assert!(!m.soins.mort && !m.acteurs[0].fantome);
        assert!(!m.objets.iter().any(|o| o.t == TypeObjet::Tombe));
    }
}
