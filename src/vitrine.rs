//! Images du README, dessinées directement à partir des sprites (`pixelpal.exe --vitrine assets`) :
//! logo, bannière, moments clés, menu de soins, tenues et planche des animations.

use crate::menu::Menu;
use crate::scene::{self, NUIT, OR};
use crate::soins::Soins;
use crate::sprites::{self, Sprites};
use crate::toile::Toile;

fn agrandir(t: &Toile, k: i32) -> Toile {
    let mut g = Toile::new(t.w * k, t.h * k);
    g.coller(t, 0.0, 0.0, k as f32, false, 1.0);
    g
}

/// Petit décor nocturne de 64 × 60 avec un sol d'herbe en y = 52 ; le titre est écrit dans l'herbe.
fn decor(titre: &str) -> Toile {
    let mut t = Toile::new(64, 60);
    for y in 0..60 {
        let k = y as f32 / 60.0;
        let c = |a: u32, b: u32| ((a as f32) * (1.0 - k) + (b as f32) * k) as u32;
        let (r, g, b) = (c(0x14, 0x3A), c(0x0C, 0x24), c(0x30, 0x6A));
        t.rect(0xFF00_0000 | (r << 16) | (g << 8) | b, 0, y, 64, 1);
    }
    for (x, y) in [(7, 14), (21, 9), (50, 12), (58, 22), (36, 18), (13, 27)] { t.rect(0xFFE9DCFF, x, y, 1, 1); }
    t.rect(0xFF2C3A2A, 0, 52, 64, 8);
    t.rect(scene::MOUSSE, 0, 52, 64, 1);
    for x in (2..64).step_by(5) { t.rect(0xFF5C9A45, x, 51, 1, 1); }
    let mut e = Toile::new(64, 12);
    scene::ecrire(&mut e, titre, 2, 3, OR, None);
    e.contour(NUIT);
    t.coller(&e, 1.0, 48.0, 1.0, false, 1.0);
    t
}

fn perso(t: &mut Toile, s: &mut Sprites, anim: &str, i: usize, x: i32, pied: i32, miroir: bool) {
    let img = s.image_n("aventurier", anim, i).clone();
    t.coller(&img, (x - sprites::CX) as f32, (pied - sprites::BAS - 1) as f32, 1.0, miroir, 1.0);
}

pub fn generer(dossier: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dossier)?;
    let ecrire = |nom: &str, t: &Toile| crate::ecrire_png(&format!("{dossier}/{nom}"), t);
    let mut s = Sprites::default();

    // logo : la tête de l'aventurier
    let img = s.image_n("aventurier", "idle", 0).clone();
    let mut tete = Toile::new(20, 20);
    tete.coller(&img, -10.0, -2.0, 1.0, false, 1.0);
    ecrire("logo.png", &agrandir(&tete, 10));

    // bannière du panneau
    let mut b = Toile::new(scene::SW, scene::SH);
    scene::scene(&mut b, 5.0, &mut s, "aventurier", crate::maj::VERSION);
    ecrire("banniere.png", &agrandir(&b, 4));

    // moments clés, en grille 3 × 2
    let mut grille = Toile::new(64 * 3 + 4, 60 * 2 + 2);
    grille.rect(NUIT, 0, 0, grille.w, grille.h);
    let mut cases = vec![];

    let mut t = decor("PARACHUTE");
    sprites::voile(&mut t, 1, 32, 18, 0.3);
    perso(&mut t, &mut s, "parachute", 0, 32, 46, false);
    cases.push(t);

    let mut t = decor("ÉCHELLE");
    sprites::echelle(&mut t, 1, 34, 10, 52, 1.0, false);
    perso(&mut t, &mut s, "climb", 1, 33, 40, false);
    cases.push(t);

    let mut t = decor("PORTAIL");
    sprites::portail(&mut t, 1, 46, 52, 1.0, 0.7);
    perso(&mut t, &mut s, "portal", 1, 20, 52, false);
    cases.push(t);

    let mut t = decor("MIAM !");
    perso(&mut t, &mut s, "eat_pomme", 0, 30, 52, false);
    for (dx, dy, a) in [(2, 12, 1.0), (9, 7, 0.7), (-4, 4, 0.4)] { sprites::coeur(&mut t, 1, 32 + dx, dy + 2, a); }
    cases.push(t);

    let mut t = decor("AÏE !");
    let ko = s.image_n("aventurier", "ko", 0).pivoter_gauche();
    t.coller(&ko, 12.0, 28.0, 1.0, false, 1.0);
    for (dx, dy) in [(20, 42), (27, 39), (15, 38)] { sprites::etoile(&mut t, 1, dx, dy); }
    cases.push(t);

    let mut t = decor("FANTÔME");
    sprites::tombe(&mut t, 1, 22, 52);
    let fantome = s.image_n("aventurier", "idle", 0).fantome(0.6);
    t.coller(&fantome, (42 - sprites::CX) as f32, (44 - sprites::BAS - 1) as f32, 1.0, true, 1.0);
    cases.push(t);

    for (i, c) in cases.iter().enumerate() {
        let (x, y) = ((i as i32 % 3) * 66, (i as i32 / 3) * 62);
        grille.coller(c, x as f32, y as f32, 1.0, false, 1.0);
    }
    ecrire("moments.png", &agrandir(&grille, 4));

    // menu de soins
    let soins = Soins { satiete: 72.0, sante: 95.0, joie: 64.0, pomme: 3, baie: 2, champi: 1, ..Default::default() };
    let mut menu = Menu::new(crate::monde::MOI, "Indy", &soins);
    menu.survol = Some(0);
    ecrire("menu.png", &agrandir(&menu.rendre(), 3));

    // tenues
    let mut tenues = Toile::new(40 * sprites::LOOKS.len() as i32, 40);
    for (i, (look, _)) in sprites::LOOKS.iter().enumerate() {
        let img = s.image_n(look, "idle", 0).clone();
        tenues.coller(&img, (i as i32 * 40) as f32, 0.0, 1.0, false, 1.0);
    }
    ecrire("tenues.png", &agrandir(&tenues, 4));

    ecrire("planche.png", &sprites::planche("aventurier", 3));
    Ok(())
}
