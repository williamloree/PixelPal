//! Simulation : écrans, passages d'un écran à l'autre, acteurs (le personnage et les visiteurs), décors.
//! Toutes les coordonnées sont en pixels physiques de l'écran virtuel de Windows.

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
pub enum TypeObjet { Echelle, Trou, Portail }

#[derive(Clone, Debug)]
pub struct Objet {
    pub id: u64,
    pub t: TypeObjet,
    pub cle: String,
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
    OuvrirPanneau,
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
    Echelle { cle: String, x: f32, y_haut: f32, y_bas: f32, vers_bas: bool, id: u64 },
    Creuser { cle: String, x: f32, y: f32, id: u64 },
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
}

impl Acteur {
    fn new(id: u64, nom: &str, look: &str, x: f32, y: f32, ecran: usize) -> Self {
        Acteur { id, nom: nom.into(), look: look.into(), x, y, sens: 1, anim: "idle", anim_t: 0.0, visible: true, echelle: 1.0, ecran,
            visiteur: false, bulle: None, msg: None, file: VecDeque::new(), action: None, neuf: false, portail: None, lettre: false,
            transit: false, occupe: false, lu: false, resultat: None, vy: 0.0, supprimer: false }
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
    pub k: f32, // facteur d'échelle (DPI de l'écran principal)
    pub temps: f32,
    seq: u64,
    alea: u64,
}

fn clamp(v: f32, a: f32, b: f32) -> f32 { if a > b { (a + b) / 2.0 } else { v.max(a).min(b) } }

impl Monde {
    pub fn new(nom: &str, look: &str) -> Self {
        let graine = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        let mut m = Monde { ecrans: vec![], liens: vec![], acteurs: vec![], objets: vec![], sorties: vec![], k: 1.0, temps: 0.0, seq: 10, alea: graine | 1 };
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
        self.liens = calculer_liens(&ecrans, self.px(12.0));
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
        }
        let ecrans = self.ecrans.clone();
        for a in &mut self.acteurs {
            let e = ecrans.iter().find(|e| e.id == a.ecran).unwrap();
            a.y = e.sol;
            a.x = clamp(a.x, e.gx, e.dx);
            if a.visiteur { a.file.push_back(Action::AttendreQue(Cond::Lu, "deliver")); a.file.extend(Self::depart_visiteur()); }
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

    fn trouver(&mut self, t: TypeObjet, cle: &str) -> Option<(f32, u64)> {
        let o = self.objets.iter_mut().find(|o| o.t == t && o.cle == cle && o.vie > 8.0)?;
        o.vie = 90.0;
        Some((o.x, o.id))
    }

    /// Prépare le trajet du personnage jusqu'à l'écran `cible`.
    fn aller_a(&mut self, ai: usize, cible: usize) {
        let de = self.acteurs[ai].ecran;
        let Some(ch) = self.chemin(de, cible) else { return self.voyage_portail(ai, cible); };
        if ch.is_empty() { return; }
        let mut l_actions = vec![Action::Transit(true)];
        let mut x = self.acteurs[ai].x;
        let p = |m: &Monde, v: f32| m.px(v);
        for l in ch {
            let (ea, eb) = (self.ecran(l.de).clone(), self.ecran(l.vers).clone());
            let cle_v = format!("v{}-{}", l.de.min(l.vers), l.de.max(l.vers));
            let cle_s = format!("s{}-{}", l.de.min(l.vers), l.de.max(l.vers));
            match l.t {
                TypeLien::Cote { sens: s, x: bord } => {
                    let sf = s as f32;
                    let (fa, fb) = (ea.sol, eb.sol);
                    let diff = fb - fa;
                    let ech = self.trouver(TypeObjet::Echelle, &cle_s);
                    if diff.abs() <= p(self, 8.0) {
                        l_actions.extend([Action::Marcher(bord + sf * p(self, 70.0)), Action::PoserY(fb), Action::Ecran(eb.id)]);
                    } else if diff < 0.0 {
                        // l'écran d'arrivée est plus haut : échelle contre le bord, côté départ
                        let lx = ech.map(|e| e.0).unwrap_or(bord - sf * p(self, 30.0));
                        if ech.is_none() {
                            l_actions.extend([Action::Marcher(lx - sf * p(self, 45.0)), Action::Sens(s),
                                Action::Echelle { cle: cle_s, x: lx, y_haut: fb - p(self, 30.0), y_bas: fa, vers_bas: false, id: 0 }]);
                        }
                        l_actions.extend([Action::Marcher(lx), Action::Grimper(fb, lx), Action::Ecran(eb.id), Action::Marcher(bord + sf * p(self, 70.0))]);
                    } else if diff <= p(self, 160.0) && ech.is_none() {
                        // petite marche : il saute
                        l_actions.extend([Action::Marcher(bord - sf * p(self, 15.0)),
                            Action::Sauter { x: bord + sf * p(self, 80.0), y: fb, t: -1.0, x0: 0.0, y0: 0.0 },
                            Action::Ecran(eb.id), Action::Attendre(0.3, "land")]);
                    } else {
                        // grande descente : il pose une échelle depuis le haut, de l'autre côté du bord
                        let lx = ech.map(|e| e.0).unwrap_or(bord + sf * p(self, 30.0));
                        l_actions.push(Action::Marcher(bord - sf * p(self, 20.0)));
                        if ech.is_none() {
                            l_actions.extend([Action::Sens(s), Action::Echelle { cle: cle_s, x: lx, y_haut: fa - p(self, 30.0), y_bas: fb, vers_bas: true, id: 0 }]);
                        }
                        l_actions.extend([Action::Marcher(lx), Action::Grimper(fb, lx), Action::Ecran(eb.id), Action::Marcher(lx + sf * p(self, 50.0))]);
                    }
                    x = bord + sf * p(self, 70.0);
                }
                TypeLien::Bas { x0, x1 } => {
                    let hx = clamp(x, x0 + p(self, 70.0), x1 - p(self, 70.0));
                    if let Some((ex, _)) = self.trouver(TypeObjet::Echelle, &cle_v) {
                        l_actions.extend([Action::Marcher(ex), Action::Grimper(eb.sol, ex), Action::Ecran(eb.id), Action::Marcher(ex + p(self, 50.0))]);
                        x = ex + p(self, 50.0);
                    } else {
                        let tx = match self.trouver(TypeObjet::Trou, &cle_v) {
                            Some((tx, _)) => { l_actions.push(Action::Marcher(tx)); tx }
                            None => { l_actions.extend([Action::Marcher(hx), Action::Creuser { cle: cle_v, x: hx, y: ea.sol, id: 0 }]); hx }
                        };
                        l_actions.extend([Action::Tomber(eb.sol), Action::Ecran(eb.id), Action::Attendre(0.35, "land")]);
                        x = tx;
                    }
                }
                TypeLien::Haut { x0, x1 } => {
                    let hx = clamp(x, x0 + p(self, 70.0), x1 - p(self, 70.0));
                    let lx = match self.trouver(TypeObjet::Echelle, &cle_v) {
                        Some((ex, _)) => { l_actions.push(Action::Marcher(ex)); ex }
                        None => {
                            l_actions.extend([Action::Marcher(hx), Action::Echelle { cle: cle_v, x: hx, y_haut: eb.sol - p(self, 30.0), y_bas: ea.sol, vers_bas: false, id: 0 }]);
                            hx
                        }
                    };
                    l_actions.extend([Action::Grimper(eb.sol, lx), Action::Ecran(eb.id), Action::Marcher(lx + p(self, 50.0))]);
                    x = lx + p(self, 50.0);
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
        if id == MOI { self.sorties.push(Sortie::OuvrirPanneau); return; }
        if let Some(a) = self.acteurs.iter_mut().find(|a| a.id == id) {
            if a.visiteur && !a.lu {
                a.lu = true;
                if let Some(m) = a.msg.clone() { self.sorties.push(Sortie::AccuserLecture(m)); }
            }
        }
    }

    // ---------- promenade ----------
    fn flaner(&mut self, ai: usize) {
        let e = self.ecran(self.acteurs[ai].ecran).clone();
        let autres: Vec<usize> = self.ecrans.iter().map(|x| x.id).filter(|&i| i != e.id && self.chemin(e.id, i).is_some()).collect();
        let r = self.hasard();
        if !autres.is_empty() && r < 0.22 {
            let c = autres[(self.hasard() * autres.len() as f32) as usize % autres.len()];
            self.aller_a(ai, c);
            let ec = self.ecran(c).clone();
            let x = self.entre(ec.gx, ec.dx);
            self.acteurs[ai].file.push_back(Action::Marcher(x));
        } else if r < 0.6 {
            let x = self.entre(e.gx, e.dx);
            self.acteurs[ai].file.push_back(Action::Marcher(x));
        } else if r < 0.72 {
            let d = self.entre(4.0, 9.0);
            self.acteurs[ai].file.push_back(Action::Attendre(d, "sit"));
        } else if r < 0.78 {
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
            } else {
                o.vie -= dt;
            }
        }
        self.objets.retain(|o| if o.t == TypeObjet::Portail { !(o.cible == 0.0 && o.p <= 0.0) } else { o.vie > -2.0 });

        for ai in 0..self.acteurs.len() {
            {
                let a = &mut self.acteurs[ai];
                a.anim_t += dt;
                if let Some(b) = &mut a.bulle { if let Some(d) = &mut b.duree { *d -= dt; if *d <= 0.0 { a.bulle = None; } } }
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
                let a = &mut self.acteurs[ai];
                let m = a.marche();
                a.anim(m);
                let d = *x - a.x;
                if d.abs() < 1.5 { a.x = *x; return true; }
                a.sens = d.signum() as i32;
                a.x += d.signum() * d.abs().min(vitesse * dt);
                false
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
            Action::Echelle { cle, x, y_haut, y_bas, vers_bas, id } => {
                if neuf {
                    *id = self.nid();
                    self.objets.push(Objet { id: *id, t: TypeObjet::Echelle, cle: cle.clone(), x: *x, y: 0.0, y_haut: *y_haut, y_bas: *y_bas, vers_bas: *vers_bas, p: 0.0, cible: 1.0, vie: 90.0 });
                    self.acteurs[ai].anim("build");
                }
                let duree = (2.0 * (*y_bas - *y_haut) / self.px(400.0)).clamp(1.5, 5.0);
                let Some(o) = self.objets.iter_mut().find(|o| o.id == *id) else { return true; };
                o.p += dt / duree;
                o.p >= 1.0
            }
            Action::Creuser { cle, x, y, id } => {
                if neuf {
                    *id = self.nid();
                    self.objets.push(Objet { id: *id, t: TypeObjet::Trou, cle: cle.clone(), x: *x, y: *y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 0.0, cible: 1.0, vie: 90.0 });
                    self.acteurs[ai].anim("dig");
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
                    self.objets.push(Objet { id, t: TypeObjet::Portail, cle: String::new(), x, y, y_haut: 0.0, y_bas: 0.0, vers_bas: false, p: 0.0, cible: 1.0, vie: 999.0 });
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
    fn ecran_dessus_echelle_puis_reutilisee() {
        let ecrans = vec![ecran(0, 0.0, 0.0, 1920.0, 1080.0), ecran(1, 0.0, -1080.0, 1920.0, 1080.0)];
        let (mut m, anims) = voyage(ecrans, 1);
        arrive(&m, 1);
        assert!(anims.contains(&"build") && anims.contains(&"climb"), "{anims:?}");
        // retour : il redescend par la même échelle au lieu de creuser
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
        assert!(vu.contains(&"climb") && !vu.contains(&"dig"), "{vu:?}");
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
}
