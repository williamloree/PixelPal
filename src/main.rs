#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fenetres;
mod maj;
mod menu;
mod monde;
mod pair;
mod panneau;
mod reseau;
mod scene;
mod soins;
mod sprites;
mod toile;
mod vitrine;

use config::{Config, Groupe, Membre};
use fenetres::Calque;
use menu::Menu;
use monde::{Monde, MOI, Sortie, TypeObjet};
use reseau::{Evt, Identite, Reseau};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use toile::Toile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |nom: &str| args.iter().position(|a| a == nom).and_then(|i| args.get(i + 1)).cloned();
    if args.iter().any(|a| a == "--planche") {
        let sortie = arg("--planche").unwrap_or_else(|| "planche.png".into());
        let look = arg("--look").unwrap_or_else(|| "aventurier".into());
        return ecrire_png(&sortie, &sprites::planche(&look, 4));
    }
    if args.iter().any(|a| a == "--scene") {
        // aperçu de la scène du panneau, à quatre instants
        let mut s = sprites::Sprites::default();
        let mut planche = Toile::new(scene::SW * 3, scene::SH * 3 * 4);
        for (i, tm) in [1.0f32, 2.3, 4.1, 7.7].iter().enumerate() {
            let mut t = Toile::new(scene::SW, scene::SH);
            scene::scene(&mut t, *tm, &mut s, "aventurier", maj::VERSION);
            planche.coller(&t, 0.0, (i as i32 * scene::SH * 3) as f32, 3.0, false, 1.0);
        }
        return ecrire_png(&arg("--scene").unwrap_or_else(|| "scene.png".into()), &planche);
    }
    if args.iter().any(|a| a == "--vitrine") {
        // images du README
        let _ = vitrine::generer(&arg("--vitrine").unwrap_or_else(|| "assets".into()));
        return;
    }
    if args.iter().any(|a| a == "--panneau") {
        let port: u16 = arg("--panneau").and_then(|p| p.parse().ok()).unwrap_or(reseau::PORT_TCP);
        let _ = panneau::lancer(port, arg("--jeton").unwrap_or_default());
        return;
    }
    let profil = args.iter().find_map(|a| a.strip_prefix("--profil=")).map(String::from);
    lancer(profil);
}

fn ecrire_png(chemin: &str, t: &Toile) {
    let f = std::fs::File::create(chemin).expect("création du PNG");
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), t.w as u32, t.h as u32);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&t.vers_rgba()).unwrap();
}

struct Envoi { dest: Vec<Membre>, msg: Value }

struct Appli {
    cfg: Config,
    profil: Option<String>,
    monde: Monde,
    reseau: Arc<Reseau>,
    jeton: String,
    envois: HashMap<u64, Envoi>,
    n_envoi: u64,
    panneau: Option<std::process::Child>,
    quitter: bool,
    etat_maj: Arc<Mutex<String>>,
    verifier_maj: Sender<()>,
}

/// Fil de mise à jour : vérifie au démarrage, toutes les 6 heures, ou à la demande du panneau.
fn fil_maj(etat: Arc<Mutex<String>>, demande: Receiver<()>, prete: Sender<String>) {
    if !maj::active() {
        *etat.lock().unwrap() = "Mises à jour désactivées (version compilée à la main)".into();
        return;
    }
    std::thread::spawn(move || loop {
        *etat.lock().unwrap() = "Recherche d'une nouvelle version…".into();
        let r = maj::verifier().and_then(|n| match n {
            None => Ok(format!("À jour (vérifié à {})", heure())),
            Some(n) => {
                *etat.lock().unwrap() = format!("Téléchargement de la version {}…", n.version);
                maj::installer(&n)?;
                let _ = prete.send(n.version.clone());
                Ok(format!("Version {} installée, redémarrage…", n.version))
            }
        });
        *etat.lock().unwrap() = r.unwrap_or_else(|e| format!("Échec de la vérification : {e}"));
        let _ = demande.recv_timeout(Duration::from_secs(6 * 3600));
    });
}

fn heure() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{:02}:{:02} UTC", s / 3600 % 24, s / 60 % 60)
}

/// Passage à iroh : l'identifiant devient le code d'invitation. Les anciens contacts (adresse:port)
/// ne sont plus joignables ; on les retire des contacts et des groupes.
fn migrer_identite(cfg: &mut Config, code: &str) {
    if cfg.id == code { return; }
    let ancien = std::mem::replace(&mut cfg.id, code.to_string());
    cfg.contacts.retain(|m| reseau::code_valide(&m.id));
    for g in &mut cfg.groupes {
        for m in &mut g.membres { if m.id == ancien { m.id = code.to_string(); } }
        g.membres.retain(|m| reseau::code_valide(&m.id));
    }
    cfg.groupes.retain(|g| g.membres.len() > 1);
    cfg.sauver();
}

/// Le personnage est tenu (ou sur le point de l'être) à la souris.
struct Saisie { id: u64, x0: f32, y0: f32, glisse: bool, trace: VecDeque<(Instant, f32, f32)> }

fn lancer(profil: Option<String>) {
    let apres_maj = std::env::args().any(|a| a == "--apres-maj");
    if !fenetres::verrou_instance(&profil, apres_maj) { return; } // déjà lancé pour ce profil
    maj::nettoyer();
    let cfg = Config::charger(&profil);
    // jeton du canal local du panneau ; PIXELPAL_JETON le fixe (tests automatiques)
    let jeton = std::env::var("PIXELPAL_JETON").ok().filter(|j| j.len() >= 8).unwrap_or_else(config::id_aleatoire);
    let (tx, rx) = channel::<Evt>();
    let mut cfg = cfg;
    let reseau = match Reseau::demarrer(&config::dossier(&profil), Identite { id: cfg.id.clone(), nom: cfg.nom.clone(), look: cfg.look.clone() }, &cfg.contacts, jeton.clone(), tx) {
        Ok(r) => r,
        Err(e) => { fenetres::alerte(&format!("PixelPal ne peut pas démarrer son réseau : {e}")); return; }
    };
    migrer_identite(&mut cfg, &reseau.code());
    let (tx_res, rx_res) = channel::<String>();
    let etat_maj = Arc::new(Mutex::new(String::new()));
    let (verifier_maj, demande_maj) = channel::<()>();
    let (tx_prete, rx_prete) = channel::<String>();
    fil_maj(etat_maj.clone(), demande_maj, tx_prete);
    let mut relance: Option<Instant> = None;

    fenetres::initialiser();
    let icone = fenetres::Icone::new();
    let mut monde = Monde::new(&cfg.nom, &cfg.look);
    monde.soins = cfg.soins.clone();
    if let Some(v) = std::env::var("PIXELPAL_VITESSE_SOINS").ok().and_then(|v| v.parse().ok()) { monde.vitesse_soins = v; }
    let (ecrans, k) = fenetres::ecrans();
    monde.definir_ecrans(ecrans, k);
    if apres_maj { monde.dire(MOI, &format!("Me revoilà, en version {} !", maj::VERSION), 5.0); }

    let mut app = Appli { cfg, profil, monde, reseau, jeton, envois: HashMap::new(), n_envoi: 0, panneau: None, quitter: false, etat_maj, verifier_maj };
    let mut sprites = sprites::Sprites::default();
    let mut calques: HashMap<String, Calque> = HashMap::new();
    let mut dernier = Instant::now();
    let mut verif_ecrans = Instant::now();
    let mut verif_fenetres = Instant::now();
    let mut sauvegarde = Instant::now();
    let mut saisie: Option<Saisie> = None;
    let mut menu: Option<Menu> = None;

    while fenetres::pomper() && !app.quitter {
        let debut = Instant::now();
        let dt = dernier.elapsed().as_secs_f32().min(0.1);
        dernier = debut;
        let u = app.monde.u();

        // clics sur les fenêtres
        let clics: Vec<(isize, i32, i32)> = std::mem::take(&mut *fenetres::CLICS.lock().unwrap());
        for (h, cx, cy) in clics {
            if h == icone.hwnd.0 as isize { app.ouvrir_panneau(); continue; }
            if calques.get("menu").is_some_and(|c| c.hwnd.0 as isize == h) {
                let choix = menu.as_ref().and_then(|m| m.choix_a(cy / Menu::echelle(u)).and_then(|i| m.choix.get(i)).map(|c| c.cle));
                if let Some(cle) = choix { menu = None; app.action_menu(cle); }
                continue;
            }
            let _ = cx;
            menu = None; // un clic ailleurs referme le menu
            let Some((cle, _)) = calques.iter().find(|(_, c)| c.hwnd.0 as isize == h) else { continue };
            let Ok(id) = cle[1..].parse::<u64>() else { continue };
            if cle.starts_with('a') && id == MOI {
                // clic ou prise : on décidera au relâchement
                let (x, y) = fenetres::curseur();
                saisie = Some(Saisie { id, x0: x, y0: y, glisse: false, trace: VecDeque::new() });
            } else {
                app.monde.clic(id);
            }
        }

        // personnage tenu à la souris : il suit le curseur, puis il est lâché (et peut être lancé)
        if let Some(s) = &mut saisie {
            let (x, y) = fenetres::curseur();
            s.trace.push_back((Instant::now(), x, y));
            while s.trace.len() > 2 && s.trace[0].0.elapsed() > Duration::from_millis(90) { s.trace.pop_front(); }
            if !s.glisse && ((x - s.x0).powi(2) + (y - s.y0).powi(2)).sqrt() > 6.0 * app.monde.k {
                s.glisse = app.monde.saisir(s.id);
                if s.glisse { menu = None; }
            }
            if s.glisse { app.monde.tenir(s.id, x, y); }
            if !fenetres::bouton_gauche() {
                if s.glisse {
                    let (t0, x0, y0) = s.trace[0];
                    let duree = t0.elapsed().as_secs_f32().max(0.016);
                    app.monde.lacher(s.id, (x - x0) / duree, (y - y0) / duree);
                } else {
                    app.monde.clic(s.id);
                }
                saisie = None;
            }
        }

        // changement de disposition des écrans
        let change = std::mem::take(&mut *fenetres::ECRANS_CHANGES.lock().unwrap());
        if change || verif_ecrans.elapsed() > Duration::from_secs(3) {
            verif_ecrans = Instant::now();
            let (e, k) = fenetres::ecrans();
            let pareil = e.len() == app.monde.ecrans.len() && e.iter().zip(&app.monde.ecrans).all(|(a, b)| a.x == b.x && a.y == b.y && a.r == b.r && a.b == b.b && a.sol == b.sol);
            if !pareil { app.monde.definir_ecrans(e, k); }
        }
        // fenêtres ouvertes sur lesquelles il peut marcher
        if verif_fenetres.elapsed() > Duration::from_millis(250) {
            verif_fenetres = Instant::now();
            let p = if app.cfg.fenetres { fenetres::plateformes() } else { vec![] };
            app.monde.definir_plateformes(p);
            // on garde la dernière appli utilisée (le panneau et le bureau ne comptent pas)
            if let Some(t) = fenetres::titre_actif() { app.monde.titre_actif = Some(t); }
        }

        while let Ok(evt) = rx.try_recv() { app.evenement(evt); }
        while let Ok(r) = rx_res.try_recv() { app.monde.resultat_envoi(r); }
        if let Ok(v) = rx_prete.try_recv() {
            app.monde.dire(MOI, &format!("Nouvelle version {v} ! Je reviens tout de suite…"), 5.0);
            relance = Some(Instant::now() + Duration::from_secs(4));
        }
        if relance.is_some_and(|r| Instant::now() >= r) { app.quitter = true; }
        if *fenetres::QUITTER.lock().unwrap() { app.quitter = true; }
        if sauvegarde.elapsed() > Duration::from_secs(30) { sauvegarde = Instant::now(); app.sauver(); }

        app.monde.maj(dt);
        for s in std::mem::take(&mut app.monde.sorties) {
            match s {
                Sortie::Menu(id) => {
                    let nom = app.cfg.nom.clone();
                    menu = if menu.as_ref().is_some_and(|m| m.acteur == id) { None } else { Some(Menu::new(id, &nom, &app.monde.soins)) };
                }
                Sortie::Envoyer(n) => app.lancer_envoi(n, tx_res.clone()),
                Sortie::AccuserLecture(m) => app.accuser_lecture(m),
            }
        }
        // le menu suit le personnage, se met à jour, et se referme tout seul
        if let Some(m) = &mut menu {
            let tenu = app.monde.acteurs.iter().any(|a| a.id == m.acteur && a.porte);
            if tenu || m.ouvert.elapsed() > Duration::from_secs(15) { menu = None; } else { m.maj(&app.monde.soins); }
        }

        dessiner(&app.monde, &mut sprites, &mut calques, menu.as_mut(), app.cfg.devant);

        let ecoule = debut.elapsed();
        if ecoule < Duration::from_millis(33) { std::thread::sleep(Duration::from_millis(33) - ecoule); }
    }
    app.sauver();
    if let Some(mut p) = app.panneau.take() { let _ = p.kill(); }
    if relance.is_some() {
        drop(calques);
        drop(icone);
        maj::relancer();
    }
}

/// Toile d'un personnage (avec voile de parachute, étoiles, cœurs) et point d'ancrage des pieds dans la toile.
fn toile_acteur(a: &monde::Acteur, sprites: &mut sprites::Sprites, u: i32, temps: f32) -> (Toile, i32, i32) {
    let mut t = Toile::new(60 * u, 80 * u);
    let mut img = sprites.image(&a.look, a.anim, a.anim_t).clone();
    if a.fantome { img = img.fantome(0.55); }
    let (cx, uf) = (30 * u, u as f32);
    let miroir = a.sens < 0;
    if a.renverse {
        // au plafond : image retournée, la semelle collée au bord du haut
        t.coller(&img.retourner_v(), (cx - sprites::CX * u) as f32, (3 * u) as f32, uf, miroir, 1.0);
        return (t, cx, 4 * u);
    }
    let pied = 76 * u;
    let etoiles = |t: &mut Toile, hx: i32, hy: i32| for i in 0..3 {
        let ang = temps * 4.0 + i as f32 * 2.1;
        sprites::etoile(t, u, hx + (ang.cos() * 7.0 * uf) as i32, hy + (ang.sin() * 2.5 * uf) as i32);
    };
    if a.couche {
        // allongé sur le dos (KO) ; tête du côté opposé aux pieds
        t.coller(&img.pivoter_gauche(), (cx - 20 * u) as f32, (pied - 28 * u) as f32, uf, miroir, 1.0);
        if a.etoiles > 0.0 {
            let hx = if miroir { cx + 20 * u - 13 * u } else { cx - 20 * u + 13 * u };
            etoiles(&mut t, hx, pied - 17 * u);
        }
        return (t, cx, pied);
    }
    let k = uf * a.echelle;
    let flotte = if a.fantome { ((temps * 2.0).sin() * 3.0 + 8.0) * uf } else { 0.0 };
    t.coller(&img, cx as f32 - sprites::CX as f32 * k, pied as f32 - (sprites::BAS + 1) as f32 * k - flotte, k, miroir, 1.0);
    if a.parachute { sprites::voile(&mut t, u, cx, pied - 30 * u, (temps * 2.0).sin()); }
    if a.etoiles > 0.0 { etoiles(&mut t, cx, pied - 38 * u); }
    if a.coeurs > 0.0 {
        for i in 0..3 {
            let ph = ((2.5 - a.coeurs) * 0.8 + i as f32 * 0.33).rem_euclid(1.0);
            let x = cx + ((ph * 6.0 + i as f32).sin() * 6.0 * uf) as i32 - 2 * u;
            sprites::coeur(&mut t, u, x, pied - 40 * u - (ph * 18.0 * uf) as i32, 1.0 - ph);
        }
    }
    (t, cx, pied)
}

/// Met à jour les petites fenêtres. Le personnage vit derrière les fenêtres des applis (au niveau du bureau),
/// sauf option « devant » ou quand on le tient ; les visiteurs et le menu restent toujours visibles.
fn dessiner(m: &Monde, sprites: &mut sprites::Sprites, calques: &mut HashMap<String, Calque>, menu: Option<&mut Menu>, devant_opt: bool) {
    let u = m.u();
    let mut vus = HashSet::new();
    let (mut devant, mut derriere_decor, mut derriere_perso, mut derriere_bulles) = (vec![], vec![], vec![], vec![]);
    let moi_devant = devant_opt || m.acteurs.iter().any(|a| a.id == MOI && a.porte);
    for o in &m.objets {
        let cle = format!("o{}", o.id);
        let (gauche, haut, t) = match o.t {
            TypeObjet::Echelle => {
                let (g, h) = (o.x as i32 - 10 * u, o.y_haut as i32 - 2 * u);
                let mut t = Toile::new(20 * u, (o.y_bas - o.y_haut) as i32 + 4 * u);
                sprites::echelle(&mut t, u, o.x as i32 - g, o.y_haut as i32 - h, o.y_bas as i32 - h, o.p, o.vers_bas);
                (g, h, t)
            }
            TypeObjet::Trou => {
                let (g, h) = (o.x as i32 - 16 * u, o.y as i32 - 8 * u);
                let mut t = Toile::new(40 * u, 12 * u);
                sprites::trou(&mut t, u, o.x as i32 - g, o.y as i32 - h, o.p);
                (g, h, t)
            }
            TypeObjet::Portail => {
                let (g, h) = (o.x as i32 - 12 * u, o.y as i32 - 40 * u);
                let mut t = Toile::new(24 * u, 42 * u);
                sprites::portail(&mut t, u, o.x as i32 - g, o.y as i32 - h, o.p, m.temps);
                (g, h, t)
            }
            TypeObjet::Tombe => {
                let (g, h) = (o.x as i32 - 8 * u, o.y as i32 - 17 * u);
                let mut t = Toile::new(16 * u, 18 * u);
                sprites::tombe(&mut t, u, 8 * u, 17 * u);
                (g, h, t)
            }
            TypeObjet::Buee => {
                let (g, h) = (o.x as i32 - 10 * u, o.y as i32 - 8 * u);
                let mut t = Toile::new(20 * u, 16 * u);
                sprites::buee(&mut t, u, o.x as i32 - g, o.y as i32 - h, o.p);
                (g, h, t)
            }
            TypeObjet::Caillou => {
                let (g, h) = (o.x as i32 - 4 * u, o.y as i32 - 5 * u);
                let mut t = Toile::new(8 * u, 6 * u);
                sprites::caillou(&mut t, u, o.x as i32 - g, o.y as i32 - h);
                (g, h, t)
            }
            TypeObjet::Eclat => {
                let (g, h) = (o.x as i32 - 23 * u, o.y as i32 - 23 * u);
                let mut t = Toile::new(46 * u, 46 * u);
                sprites::eclat(&mut t, u, o.x as i32 - g, o.y as i32 - h, o.p);
                (g, h, t)
            }
        };
        // la buée (sur la vitre) et les éclats passent devant le personnage
        let par_dessus = matches!(o.t, TypeObjet::Buee | TypeObjet::Eclat);
        let mut t = t;
        let a = o.alpha();
        if a < 1.0 { for p in &mut t.px { *p = (*p & 0x00FF_FFFF) | ((((*p >> 24) as f32 * a) as u32) << 24); } }
        let c = calques.entry(cle.clone()).or_insert_with(|| Calque::new(false));
        c.afficher(&t, gauche, haut);
        if moi_devant { devant.push(c.hwnd) } else if par_dessus { derriere_bulles.push(c.hwnd) } else { derriere_decor.push(c.hwnd) }
        vus.insert(cle);
    }
    for a in &m.acteurs {
        if !a.visible || a.echelle < 0.02 { continue; }
        let cle = format!("a{}", a.id);
        let (t, ax, ay) = toile_acteur(a, sprites, u, m.temps);
        let c = calques.entry(cle.clone()).or_insert_with(|| Calque::new(true));
        c.afficher(&t, a.x as i32 - ax, a.y as i32 - ay);
        if a.visiteur || moi_devant { devant.insert(0, c.hwnd) } else { derriere_perso.push(c.hwnd) }
        vus.insert(cle);
    }
    for a in &m.acteurs {
        if !a.visible || a.echelle < 0.6 || a.renverse { continue; }
        let (titre, texte, indice) = match &a.bulle {
            Some(b) => (b.titre.clone(), b.texte.clone(), if b.cliquable { "(clic pour fermer)".to_string() } else { String::new() }),
            None if a.visiteur => (a.nom.clone(), String::new(), String::new()),
            None => continue,
        };
        let cle = format!("b{}", a.id);
        let decal = if a.couche { 16 } else if a.fantome { 46 } else if a.parachute { 50 } else { 35 };
        let c = calques.entry(cle.clone()).or_insert_with(|| Calque::new(true));
        c.bulle(&titre, &texte, &indice, a.x as i32, a.y as i32 - decal * u, m.k);
        if a.visiteur || moi_devant { devant.insert(0, c.hwnd) } else { derriere_bulles.push(c.hwnd) }
        vus.insert(cle);
    }
    if let Some(menu) = menu {
        if let Some(a) = m.acteurs.iter().find(|a| a.id == menu.acteur) {
            let c = calques.entry("menu".into()).or_insert_with(|| Calque::new(true));
            // survol : la ligne sous le curseur s'allume
            let (sx, sy) = fenetres::curseur();
            let km = Menu::echelle(u);
            let (w, h) = (Menu::largeur() * km, menu.hauteur() * km);
            let e = m.ecran(a.ecran);
            let mut x = a.x as i32 + 14 * u;
            if x + w > e.r as i32 { x = a.x as i32 - 14 * u - w; }
            let y = (a.y as i32 - 30 * u - h / 2).clamp(e.y as i32, (e.b as i32 - h).max(e.y as i32));
            menu.survol = if (sx as i32) >= x && (sx as i32) < x + w { menu.choix_a((sy as i32 - y) / km) } else { None };
            let mut t = Toile::new(w, h);
            t.coller(&menu.rendre(), 0.0, 0.0, km as f32, false, 1.0);
            c.afficher(&t, x, y);
            devant.insert(0, c.hwnd);
            vus.insert("menu".to_string());
        }
    }
    calques.retain(|c, _| vus.contains(c));
    // ordre derrière : bulles au-dessus du personnage, lui-même au-dessus de ses échelles
    let derriere: Vec<_> = derriere_bulles.into_iter().chain(derriere_perso).chain(derriere_decor).collect();
    fenetres::empiler(&devant, &derriere);
}

impl Appli {
    fn ouvrir_panneau(&mut self) {
        if let Some(p) = &mut self.panneau {
            if let Ok(None) = p.try_wait() { return; }
        }
        let exe = std::env::current_exe().unwrap();
        self.panneau = std::process::Command::new(exe)
            .args(["--panneau", &self.reseau.port.to_string(), "--jeton", &self.jeton])
            .spawn().ok();
    }

    /// Enregistre les soins et les contacts.
    fn sauver(&mut self) {
        self.cfg.soins = self.monde.soins.clone();
        self.sauver_contacts();
        self.cfg.sauver();
    }

    /// Choix fait dans le menu de soins.
    fn action_menu(&mut self, cle: &str) {
        let r = match cle {
            "panneau" => { self.ouvrir_panneau(); Ok(()) }
            "caresser" => self.monde.caresser(),
            "ressusciter" => self.monde.ressusciter(),
            aliment => self.monde.nourrir(aliment),
        };
        if let Err(e) = r { self.monde.dire(MOI, &e, 3.5); } else { self.sauver(); }
    }

    fn sauver_contacts(&mut self) {
        let c = self.reseau.contacts.lock().unwrap();
        let mut v: Vec<Membre> = c.values().filter(|c| !c.nom.is_empty()).map(|c| Membre { id: c.id.clone(), nom: c.nom.clone(), host: String::new(), port: 0 }).collect();
        drop(c);
        v.sort_by(|a, b| a.nom.cmp(&b.nom));
        let changes = v.len() != self.cfg.contacts.len() || v.iter().zip(&self.cfg.contacts).any(|(a, b)| a.id != b.id || a.host != b.host || a.port != b.port || a.nom != b.nom);
        if changes { self.cfg.contacts = v; self.cfg.sauver(); }
    }

    fn moi_membre(&self) -> Membre { Membre { id: self.cfg.id.clone(), nom: self.cfg.nom.clone(), host: String::new(), port: 0 } }

    fn ecran_du_curseur(&self) -> usize {
        let (x, y) = fenetres::curseur();
        let e = &self.monde.ecrans;
        e.iter().find(|e| x >= e.x && x < e.r && y >= e.y && y < e.b).or_else(|| e.iter().find(|e| e.principal)).map(|e| e.id).unwrap_or(0)
    }

    fn evenement(&mut self, e: Evt) {
        match e {
            Evt::Contacts => {}
            Evt::Message(v) => {
                let nom = v["from"]["name"].as_str().unwrap_or("?").to_string();
                let look = v["from"]["look"].as_str().unwrap_or("aventurier").to_string();
                let groupe = v["groupe_nom"].as_str().unwrap_or("");
                let titre = if groupe.is_empty() { format!("✉ {nom}") } else { format!("✉ {nom} · {groupe}") };
                let texte: String = v["texte"].as_str().unwrap_or("").chars().take(2000).collect();
                let ecran = self.ecran_du_curseur();
                self.monde.accueillir(&nom, &look, titre, texte, v, ecran);
            }
            Evt::Lu(v) => {
                let nom = v["from"]["name"].as_str().unwrap_or("?");
                self.monde.dire(MOI, &format!("{nom} a lu ta lettre."), 4.0);
            }
            Evt::Groupe(v) => {
                let Ok(g) = serde_json::from_value::<Groupe>(v["groupe"].clone()) else { return };
                let de = v["from"]["name"].as_str().unwrap_or("?");
                {
                    let mut c = self.reseau.contacts.lock().unwrap();
                    for m in g.membres.iter().filter(|m| m.id != self.cfg.id && !m.host.is_empty()) {
                        c.entry(m.id.clone()).or_insert(reseau::Contact { id: m.id.clone(), nom: m.nom.clone(), look: String::new(), vu: None, chemin: String::new() });
                    }
                }
                self.monde.dire(MOI, &format!("{de} m'a ajouté au groupe « {} ».", g.nom), 5.0);
                self.cfg.groupes.retain(|x| x.id != g.id);
                self.cfg.groupes.push(g);
                self.cfg.sauver();
            }
            Evt::Controle(v, rep) => { let r = self.controle(&v); let _ = rep.send(r); }
        }
    }

    /// Commandes envoyées par le panneau.
    fn controle(&mut self, v: &Value) -> Value {
        match v["cmd"].as_str().unwrap_or("") {
            "etat" => self.etat(),
            "profil" => {
                if let Some(n) = v["nom"].as_str().filter(|n| !n.trim().is_empty()) { self.cfg.nom = n.trim().chars().take(40).collect(); }
                if let Some(l) = v["look"].as_str() { self.cfg.look = l.into(); }
                self.cfg.sauver();
                let mut moi = self.reseau.moi.lock().unwrap();
                moi.nom = self.cfg.nom.clone();
                moi.look = self.cfg.look.clone();
                drop(moi);
                let (nom, look) = (self.cfg.nom.clone(), self.cfg.look.clone());
                let m = self.monde.moi();
                m.nom = nom; m.look = look;
                self.monde.dire(MOI, "Nouvelle tenue !", 2.5);
                json!({"ok": true})
            }
            "creer_groupe" => {
                let nom = v["nom"].as_str().unwrap_or("").trim().to_string();
                if nom.is_empty() { return json!({"ok": false, "erreur": "donne un nom au groupe"}); }
                let ids: Vec<String> = v["ids"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                if ids.is_empty() { return json!({"ok": false, "erreur": "choisis au moins un contact"}); }
                let c = self.reseau.contacts.lock().unwrap();
                let mut membres = vec![self.moi_membre()];
                membres.extend(ids.iter().filter_map(|id| c.get(id)).map(|c| Membre { id: c.id.clone(), nom: c.nom.clone(), host: String::new(), port: 0 }));
                drop(c);
                let g = Groupe { id: config::id_aleatoire(), nom, membres };
                self.cfg.groupes.push(g.clone());
                self.cfg.sauver();
                for m in g.membres.iter().skip(1).cloned() {
                    let (r, g2) = (self.reseau.clone(), g.clone());
                    std::thread::spawn(move || { let _ = r.envoyer_a(&m.id, json!({"type": "groupe", "groupe": g2})); });
                }
                json!({"ok": true})
            }
            "supprimer_groupe" => {
                let id = v["id"].as_str().unwrap_or("");
                self.cfg.groupes.retain(|g| g.id != id);
                self.cfg.sauver();
                json!({"ok": true})
            }
            "envoyer" => {
                let texte = v["texte"].as_str().unwrap_or("").trim().to_string();
                if texte.is_empty() { return json!({"ok": false, "erreur": "message vide"}); }
                let Some(g) = self.cfg.groupes.iter().find(|g| Some(g.id.as_str()) == v["groupe"].as_str()).cloned() else {
                    return json!({"ok": false, "erreur": "groupe introuvable"});
                };
                let choix: Option<Vec<String>> = v["a"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect());
                let dest: Vec<Membre> = g.membres.iter().filter(|m| m.id != self.cfg.id)
                    .filter(|m| choix.as_ref().map(|c| c.contains(&m.id)).unwrap_or(true)).cloned().collect();
                if dest.is_empty() { return json!({"ok": false, "erreur": "aucun destinataire"}); }
                self.n_envoi += 1;
                let msg = json!({"type": "msg", "id": config::id_aleatoire(), "groupe": g.id, "groupe_nom": g.nom, "texte": texte,
                    "a": if choix.is_some() { json!(dest.iter().map(|m| &m.id).collect::<Vec<_>>()) } else { json!("tous") }});
                self.envois.insert(self.n_envoi, Envoi { dest, msg });
                self.monde.envoyer_lettre(self.n_envoi);
                json!({"ok": true})
            }
            "demo" => {
                match v["quoi"].as_str().unwrap_or("") {
                    "ecran" => self.monde.demo_ecran(),
                    "portail" => self.monde.demo_portail(),
                    "visiteur" => {
                        let e = self.ecran_du_curseur();
                        let msg = json!({"from": {"id": "", "name": "Visiteur test"}});
                        self.monde.accueillir("Visiteur test", "violet", "✉ Visiteur test · Démo".into(),
                            "Ceci est un message de démonstration. Clique sur moi ou sur la bulle pour le fermer.".into(), msg, e);
                    }
                    q @ ("nettoyer" | "espionner" | "trebucher" | "piolet" | "artefact") => {
                        if let Err(e) = self.monde.demo(q) { return json!({"ok": false, "erreur": e}); }
                    }
                    _ => {}
                }
                json!({"ok": true})
            }
            "nourrir" | "caresser" | "ressusciter" => {
                let r = match v["cmd"].as_str().unwrap_or("") {
                    "caresser" => self.monde.caresser(),
                    "ressusciter" => self.monde.ressusciter(),
                    _ => self.monde.nourrir(v["cle"].as_str().unwrap_or("")),
                };
                match r { Ok(()) => { self.sauver(); json!({"ok": true}) } Err(e) => json!({"ok": false, "erreur": e}) }
            }
            "options" => {
                if let Some(b) = v["devant"].as_bool() { self.cfg.devant = b; }
                if let Some(b) = v["fenetres"].as_bool() { self.cfg.fenetres = b; }
                self.cfg.sauver();
                json!({"ok": true})
            }
            "quitter" => { self.quitter = true; json!({"ok": true}) }
            "maj" => {
                if !maj::active() { return json!({"ok": false, "erreur": "version compilée à la main : pas de mise à jour automatique"}); }
                let _ = self.verifier_maj.send(());
                json!({"ok": true})
            }
            _ => json!({"ok": false, "erreur": "commande inconnue"}),
        }
    }

    fn etat(&self) -> Value {
        let c = self.reseau.contacts.lock().unwrap();
        let mut contacts: Vec<Value> = c.values().map(|c| json!({"id": c.id, "nom": c.nom, "chemin": c.chemin, "en_ligne": c.en_ligne(), "look": c.look})).collect();
        contacts.sort_by(|a, b| a["nom"].as_str().cmp(&b["nom"].as_str()));
        let e = &self.monde.ecrans;
        json!({
            "profil": {"id": self.cfg.id, "nom": self.cfg.nom, "look": self.cfg.look},
            "profil_arg": self.profil,
            "version": maj::VERSION,
            "maj": *self.etat_maj.lock().unwrap(),
            "soins": self.monde.soins,
            "humeur": self.monde.soins.humeur(),
            "age": soins::duree(self.monde.soins.age),
            "record": soins::duree(self.monde.soins.record),
            "options": {"devant": self.cfg.devant, "fenetres": self.cfg.fenetres},
            "code": self.reseau.code(),
            "contacts": contacts,
            "groupes": self.cfg.groupes,
            "ecrans": e.iter().enumerate().map(|(i, e)| json!({"num": i + 1, "nom": e.nom, "x": e.x, "y": e.y, "r": e.r, "b": e.b, "sol": e.sol, "principal": e.principal})).collect::<Vec<_>>(),
            "liens": monde::decrire_liens(e, &self.monde.liens),
        })
    }

    /// Le personnage est entré dans le portail : la lettre part chez tous les destinataires en même temps.
    fn lancer_envoi(&mut self, n: u64, tx: std::sync::mpsc::Sender<String>) {
        let Some(e) = self.envois.remove(&n) else { let _ = tx.send("Lettre perdue…".into()); return; };
        let r = self.reseau.clone();
        std::thread::spawn(move || {
            let fils: Vec<_> = e.dest.iter().cloned().map(|m| {
                let (r, msg) = (r.clone(), e.msg.clone());
                std::thread::spawn(move || (m.nom.clone(), r.envoyer_a(&m.id, msg)))
            }).collect();
            let res: Vec<(String, Result<Value, String>)> = fils.into_iter().filter_map(|f| f.join().ok()).collect();
            let ok: Vec<&str> = res.iter().filter(|(_, r)| matches!(r, Ok(v) if v["ok"] == true)).map(|(n, _)| n.as_str()).collect();
            let ko: Vec<&str> = res.iter().filter(|(_, r)| !matches!(r, Ok(v) if v["ok"] == true)).map(|(n, _)| n.as_str()).collect();
            let texte = if ko.is_empty() { format!("Livré à {} ✓", ok.join(", ")) }
                else if ok.is_empty() { format!("Personne n'a répondu ({}) ✗", ko.join(", ")) }
                else { format!("Livré à {} ✓ ; absent : {}", ok.join(", "), ko.join(", ")) };
            let _ = tx.send(texte);
        });
    }

    fn accuser_lecture(&self, m: Value) {
        let Some(id) = m["from"]["id"].as_str().filter(|s| !s.is_empty()).map(String::from) else { return };
        let membre = Membre { id, nom: String::new(), host: String::new(), port: m["from"]["port"].as_u64().unwrap_or(0) as u16 };
        let r = self.reseau.clone();
        std::thread::spawn(move || {
            let _ = r.envoyer_a(&membre.id, json!({"type": "lu", "msg": m["id"]}));
        });
    }
}
