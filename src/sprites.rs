//! Le personnage en pixel art.
//!
//! Le personnage n'est pas une image figée : chaque image d'animation est une POSE
//! (angles des hanches, genoux, épaules, coudes, objet tenu…) dessinée pixel par pixel
//! sur une toile de 40 × 40, puis détourée automatiquement, calque par calque.
//! Changer de personnage = changer la palette (`LOOKS`) ou les fonctions `tete`, `chapeau`, `corps`, `sac`.
//!
//! Conventions : le personnage regarde vers la droite (l'image est retournée pour la gauche).
//! Angle 0 = membre vers le bas, positif = vers l'avant, 90 = horizontal devant, 180 = vers le haut.

use crate::toile::Toile;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const W: i32 = 40;
pub const H: i32 = 40;
pub const CX: i32 = 20;
pub const BAS: i32 = 38; // dernière ligne des pieds
const HANCHE: f32 = 27.0;
const PIED: f32 = 37.0;

// ---------- palettes ----------
#[derive(Clone, Copy)]
pub struct Palette {
    outline: u32, skin: u32, skin_shade: u32, blush: u32, hair: u32, eye: u32,
    hat: u32, hat_light: u32, hat_shade: u32, hat_band: u32,
    shirt: u32, shirt_shade: u32, scarf: u32, scarf_shade: u32,
    pants: u32, pants_shade: u32, belt: u32, buckle: u32, boots: u32, boots_shade: u32,
    pack: u32, pack_shade: u32, roll: u32, roll_shade: u32,
    wood: u32, metal: u32, metal_shade: u32, dirt: u32, paper: u32, paper_shade: u32, seal: u32,
}

const BASE: Palette = Palette {
    outline: 0xFF1D1520, skin: 0xFFF2C48D, skin_shade: 0xFFD89C62, blush: 0xFFE88F7A, hair: 0xFF4A2F1D, eye: 0xFF1D1520,
    hat: 0xFF8B5A2B, hat_light: 0xFFB07A40, hat_shade: 0xFF6A4220, hat_band: 0xFF3A2414,
    shirt: 0xFFCDB47C, shirt_shade: 0xFFA68E58, scarf: 0xFFC4372B, scarf_shade: 0xFF8E2620,
    pants: 0xFF56603D, pants_shade: 0xFF3E4729, belt: 0xFF3A2414, buckle: 0xFFE3BB4C, boots: 0xFF5D3B1F, boots_shade: 0xFF422912,
    pack: 0xFF7C4F2B, pack_shade: 0xFF5C3A1F, roll: 0xFF6F8F8C, roll_shade: 0xFF536E6B,
    wood: 0xFF9A6A35, metal: 0xFFB5BEC7, metal_shade: 0xFF6F7880, dirt: 0xFF6B4A2B, paper: 0xFFFBF3DF, paper_shade: 0xFFCFC4A8, seal: 0xFFC4372B,
};

/// Variantes de tenue : (clé, nom affiché). Le joueur choisit la sienne, les visiteurs arrivent avec la leur.
pub const LOOKS: [(&str, &str); 5] = [
    ("aventurier", "Aventurier (kaki)"),
    ("bleu", "Bleu marine"),
    ("foret", "Forêt"),
    ("rouge", "Rouge"),
    ("violet", "Mage violet"),
];

pub fn palette(look: &str) -> Palette {
    let mut p = BASE;
    match look {
        "bleu" => { p.shirt = 0xFF5F7FBF; p.shirt_shade = 0xFF46609A; p.scarf = 0xFFE3BB4C; p.scarf_shade = 0xFFB08D2C; p.hat = 0xFF3F4A5C; p.hat_light = 0xFF56637A; p.hat_shade = 0xFF2D3544; }
        "foret" => { p.shirt = 0xFF6F9A54; p.shirt_shade = 0xFF557A3F; p.scarf = 0xFFE08A3C; p.scarf_shade = 0xFFB06A28; p.pants = 0xFF6B5238; p.pants_shade = 0xFF523E2A; }
        "rouge" => { p.shirt = 0xFFC45A4A; p.shirt_shade = 0xFF9A4236; p.scarf = 0xFFF1E3B5; p.scarf_shade = 0xFFC9B98A; p.hat = 0xFF2F2A2A; p.hat_light = 0xFF463F3F; p.hat_shade = 0xFF1F1B1B; }
        "violet" => { p.shirt = 0xFF7D5BB5; p.shirt_shade = 0xFF5F438F; p.scarf = 0xFF6FE7FF; p.scarf_shade = 0xFF3FB4CC; p.hat = 0xFF4A2F7A; p.hat_light = 0xFF64449C; p.hat_shade = 0xFF35205A; p.hair = 0xFFD8D0C0; }
        _ => {}
    }
    p
}

// ---------- poses ----------
#[derive(Clone, Copy, PartialEq)]
pub enum Yeux { Ouverts, Fermes, Haut, Croix, Contents }
#[derive(Clone, Copy, PartialEq)]
pub enum Bouche { Neutre, O, Sourire }
#[derive(Clone, Copy, PartialEq)]
pub enum Objet { Rien, Lettre, Pelle, Marteau, Pomme, Baie, Champi, Mouchoir, Loupe, Jumelles, Piolet, Artefact }

#[derive(Clone, Copy)]
pub struct Pose {
    leg_f: (f32, f32), leg_b: (f32, f32), arm_f: (f32, f32), arm_b: (f32, f32),
    ub: i32, by: i32, air: bool, yeux: Yeux, bouche: Bouche, chapeau_y: i32, echarpe: i32,
    objet: Objet, obj_a: f32, terre: bool, etincelle: Option<i32>,
    objet_b: Objet, obj_b_a: f32, // objet tenu dans la main du fond (second piolet)
}

impl Default for Pose {
    fn default() -> Self {
        Pose { leg_f: (4.0, 0.0), leg_b: (-4.0, 0.0), arm_f: (-8.0, 14.0), arm_b: (10.0, 10.0), ub: 0, by: 0, air: false,
            yeux: Yeux::Ouverts, bouche: Bouche::Neutre, chapeau_y: 0, echarpe: 0, objet: Objet::Rien, obj_a: 0.0, terre: false, etincelle: None,
            objet_b: Objet::Rien, obj_b_a: 0.0 }
    }
}

fn rad(a: f32) -> f32 { a.to_radians() }
fn dir(a: f32) -> (f32, f32) { (rad(a).sin(), rad(a).cos()) }

fn px(t: &mut Toile, c: u32, x: f32, y: f32, w: i32, h: i32) { t.rect(c, x.round() as i32, y.round() as i32, w, h); }
fn trait_(t: &mut Toile, c: u32, x0: f32, y0: f32, x1: f32, y1: f32, e: i32) {
    let n = ((x1 - x0).abs().max((y1 - y0).abs())).round().max(1.0) as i32;
    for i in 0..=n {
        let k = i as f32 / n as f32;
        px(t, c, x0 + (x1 - x0) * k, y0 + (y1 - y0) * k, e, e);
    }
}

struct Jambe { kx: f32, ky: f32, ax: f32, ay: f32, sa: f32, pointe: f32 }
fn jambe(hx: f32, hy: f32, (hanche, genou): (f32, f32)) -> Jambe {
    let (a, b) = dir(hanche);
    let (kx, ky, sa) = (hx + a * 5.0, hy + b * 5.0, hanche - genou);
    let (c, d) = dir(sa);
    let (ax, ay) = (kx + c * 5.0, ky + d * 5.0);
    Jambe { kx, ky, ax, ay, sa, pointe: ay - rad(sa).sin() * 2.0 }
}

fn dessiner_jambe(t: &mut Toile, p: &Palette, hx: f32, hy: f32, angles: (f32, f32), fond: bool) {
    let j = jambe(hx, hy, angles);
    let (tissu, botte) = if fond { (p.pants_shade, p.boots_shade) } else { (p.pants, p.boots) };
    trait_(t, tissu, hx, hy, j.kx, j.ky, 2);
    trait_(t, tissu, j.kx, j.ky, j.ax, j.ay, 2);
    let (a, b) = dir(j.sa);
    trait_(t, botte, j.ax - a * 2.0, j.ay - b * 2.0, j.ax, j.ay, 2);
    trait_(t, botte, j.ax, j.ay, j.ax + rad(j.sa).cos() * 2.0, j.ay - rad(j.sa).sin() * 2.0, 2); // pointe du pied
}

struct Main { x: f32, y: f32 }
fn dessiner_bras(t: &mut Toile, p: &Palette, sx: f32, sy: f32, (epaule, coude): (f32, f32), fond: bool) -> Main {
    let (a, b) = dir(epaule);
    let (ex, ey, fa) = (sx + a * 4.0, sy + b * 4.0, epaule + coude);
    let (c, d) = dir(fa);
    let (hx, hy) = (ex + c * 3.0, ey + d * 3.0);
    let (manche, peau) = if fond { (p.shirt_shade, p.skin_shade) } else { (p.shirt, p.skin) };
    trait_(t, manche, sx, sy, ex, ey, 2);
    trait_(t, manche, ex, ey, hx, hy, 2);
    let m = Main { x: (hx + c).round(), y: (hy + d).round() };
    px(t, peau, m.x, m.y, 2, 2);
    m
}

// ---------- habillage ----------
fn sac(t: &mut Toile, p: &Palette, u: f32) {
    px(t, p.roll, 12.0, 15.0 + u, 6, 2); px(t, p.roll_shade, 12.0, 16.0 + u, 6, 1); // couverture roulée
    px(t, p.pack, 13.0, 17.0 + u, 4, 8); px(t, p.pack_shade, 13.0, 17.0 + u, 1, 8);
    px(t, p.pack_shade, 13.0, 17.0 + u, 4, 2); px(t, p.buckle, 15.0, 19.0 + u, 1, 1); // rabat et boucle
}

fn corps(t: &mut Toile, p: &Palette, u: f32, by: f32, pose: &Pose) {
    px(t, p.pants, 17.0, HANCHE - 1.0 + by, 6, 2); // bassin
    px(t, p.shirt, 17.0, 17.0 + u, 6, 8); px(t, p.shirt_shade, 17.0, 17.0 + u, 1, 8);
    px(t, p.shirt_shade, 21.0, 20.0 + u, 2, 1); // poche
    px(t, p.belt, 19.0, 18.0 + u, 1, 6); // sangle du sac
    px(t, p.belt, 17.0, 25.0 + u, 6, 1); px(t, p.buckle, 21.0, 25.0 + u, 1, 1); // ceinture
    let f = pose.echarpe as f32; // écharpe qui flotte
    px(t, p.scarf, 17.0, 16.0 + u, 6, 2); px(t, p.scarf_shade, 17.0, 17.0 + u, 2, 1);
    px(t, p.scarf, 15.0, 17.0 + u + f, 2, 1); px(t, p.scarf_shade, 14.0, 18.0 + u + f, 2, 1);
}

fn tete(t: &mut Toile, p: &Palette, u: f32, pose: &Pose) {
    let y = 9.0 + u;
    px(t, p.skin, 16.0, y, 8, 8);
    px(t, p.skin_shade, 17.0, y + 7.0, 7, 1); // menton
    px(t, p.hair, 16.0, y, 3, 6); px(t, p.hair, 16.0, y, 8, 2); px(t, p.hair, 22.0, y + 2.0, 1, 1);
    px(t, p.skin_shade, 18.0, y + 3.0, 1, 2); // oreille
    px(t, p.skin, 24.0, y + 4.0, 1, 1); // nez
    match pose.yeux {
        Yeux::Fermes => { px(t, p.eye, 20.0, y + 4.0, 1, 1); px(t, p.eye, 22.0, y + 4.0, 1, 1); }
        Yeux::Croix => {
            // yeux en croix (KO)
            for (dx, dy) in [(0.0, 0.0), (2.0, 0.0), (1.0, 1.0), (0.0, 2.0), (2.0, 2.0)] { px(t, p.eye, 19.0 + dx, y + 2.0 + dy, 1, 1); }
            px(t, p.eye, 23.0, y + 3.0, 1, 1);
        }
        Yeux::Contents => {
            // yeux plissés de bonheur (^ ^)
            for x in [20.0, 22.0] { px(t, p.eye, x, y + 3.0, 1, 1); }
            for x in [19.0, 21.0, 23.0] { px(t, p.eye, x, y + 4.0, 1, 1); }
        }
        _ => {
            let ye = if pose.yeux == Yeux::Haut { y + 2.0 } else { y + 3.0 };
            px(t, p.eye, 20.0, ye, 1, 2); px(t, p.eye, 22.0, ye, 1, 2);
        }
    }
    px(t, p.blush, 23.0, y + 5.0, 1, 1);
    match pose.bouche {
        Bouche::O => px(t, p.eye, 21.0, y + 6.0, 1, 1),
        Bouche::Sourire => { px(t, p.eye, 21.0, y + 6.0, 2, 1); px(t, p.eye, 20.0, y + 5.0, 1, 1); }
        Bouche::Neutre => px(t, p.skin_shade, 21.0, y + 6.0, 2, 1),
    }
}

fn chapeau(t: &mut Toile, p: &Palette, u: f32, pose: &Pose) {
    let y = 5.0 + u + pose.chapeau_y as f32;
    px(t, p.hat, 17.0, y, 6, 4); px(t, p.hat_light, 18.0, y, 4, 1); px(t, p.hat_shade, 19.0, y, 2, 1);
    px(t, p.hat_shade, 17.0, y + 1.0, 1, 2);
    px(t, p.hat_band, 17.0, y + 3.0, 6, 1);
    px(t, p.hat, 14.0, y + 4.0, 12, 1); px(t, p.hat_shade, 14.0, y + 4.0, 2, 1); // bord
}

fn objet(t: &mut Toile, p: &Palette, m: &Main, o: Objet, angle: f32, pose: &Pose) {
    match o {
        Objet::Lettre => {
            px(t, p.paper, m.x - 2.0, m.y - 3.0, 6, 4); px(t, p.paper_shade, m.x - 2.0, m.y, 6, 1);
            px(t, p.paper_shade, m.x - 1.0, m.y - 2.0, 1, 1); px(t, p.paper_shade, m.x + 2.0, m.y - 2.0, 1, 1);
            px(t, p.seal, m.x, m.y - 1.0, 2, 1);
        }
        Objet::Pelle => {
            let (a, b) = dir(angle);
            let (cx, cy) = (m.x + 0.5, m.y + 0.5);
            trait_(t, p.wood, cx - a * 4.0, cy - b * 4.0, cx + a * 7.0, cy + b * 7.0, 1);
            let (bx, by) = (cx + a * 8.0, cy + b * 8.0);
            px(t, p.metal, bx - 1.0, by - 1.0, 3, 3); px(t, p.metal_shade, bx - 1.0, by + 1.0, 3, 1);
            if pose.terre { px(t, p.dirt, bx - 1.0, by - 2.0, 3, 1); }
        }
        Objet::Marteau => {
            let (a, b) = dir(angle);
            trait_(t, p.wood, m.x + 0.5, m.y + 0.5, m.x + 0.5 + a * 5.0, m.y + 0.5 + b * 5.0, 1);
            px(t, p.metal, m.x + a * 5.0 - 1.0, m.y + b * 5.0 - 1.0, 3, 2);
        }
        Objet::Pomme | Objet::Baie | Objet::Champi => aliment(t, o, m.x - 1.0, m.y - 3.0),
        Objet::Mouchoir => {
            // mouchoir blanc à liseré bleu, un coin qui pend
            px(t, 0xFFF4F7FF, m.x - 1.0, m.y - 2.0, 4, 4); px(t, 0xFFD6DEEA, m.x - 1.0, m.y + 1.0, 4, 1);
            px(t, 0xFF6F9BD8, m.x - 1.0, m.y - 2.0, 4, 1); px(t, 0xFFF4F7FF, m.x, m.y + 2.0, 2, 1);
        }
        Objet::Loupe => {
            // manche, puis lentille cerclée au bout
            let (a, b) = dir(angle);
            let (cx, cy) = (m.x + 0.5, m.y + 0.5);
            trait_(t, p.wood, cx, cy, cx + a * 3.0, cy + b * 3.0, 1);
            let (lx, ly) = ((cx + a * 6.0).round(), (cy + b * 6.0).round());
            px(t, p.metal_shade, lx - 2.0, ly - 2.0, 5, 5);
            px(t, 0xFFBFEFFF, lx - 1.0, ly - 1.0, 3, 3);
            px(t, 0xFFFFFFFF, lx - 1.0, ly - 1.0, 1, 1);
        }
        Objet::Jumelles => {
            // jumelles portées aux yeux (vues de profil) : corps sombre et objectif bleuté
            px(t, 0xFF3A3440, m.x - 1.0, m.y - 4.0, 5, 3); px(t, 0xFF55505E, m.x - 1.0, m.y - 4.0, 5, 1);
            px(t, 0xFF8FD8FF, m.x + 4.0, m.y - 4.0, 1, 3); px(t, 0xFFFFFFFF, m.x + 4.0, m.y - 4.0, 1, 1);
        }
        Objet::Piolet => {
            // manche bleu, tête en acier : la pointe part vers l'avant
            let (a, b) = dir(angle);
            let (cx, cy) = (m.x + 0.5, m.y + 0.5);
            trait_(t, 0xFF3A6EA5, cx - a * 2.0, cy - b * 2.0, cx + a * 6.0, cy + b * 6.0, 1);
            let (hx, hy) = (cx + a * 6.0, cy + b * 6.0);
            let (c, d) = dir(angle - 90.0);
            trait_(t, p.metal, hx - c, hy - d, hx + c * 3.0, hy + d * 3.0, 1);
            px(t, p.metal_shade, hx + c * 3.0 + a, hy + d * 3.0 + b, 1, 1);
        }
        Objet::Artefact => {
            // gemme brillante sertie d'or, brandie au-dessus de la main
            px(t, 0xFFE3BB4C, m.x - 1.0, m.y - 5.0, 4, 5);
            px(t, 0xFF6FE7FF, m.x, m.y - 5.0, 2, 4); px(t, 0xFFE9FFFF, m.x, m.y - 5.0, 1, 1);
            px(t, 0xFF3FB4CC, m.x + 1.0, m.y - 2.0, 1, 1);
        }
        Objet::Rien => {}
    }
}

/// Petit aliment de 4 × 4 pixels (dans la main, ou en icône).
fn aliment(t: &mut Toile, o: Objet, x: f32, y: f32) {
    match o {
        Objet::Pomme => {
            px(t, 0xFFD83A2E, x, y + 1.0, 4, 3); px(t, 0xFFD83A2E, x + 1.0, y, 2, 1);
            px(t, 0xFFFF8A6A, x + 1.0, y + 1.0, 1, 1); px(t, 0xFF6FBF4F, x + 2.0, y - 1.0, 2, 1);
        }
        Objet::Baie => {
            px(t, 0xFFD16BFF, x, y + 1.0, 2, 2); px(t, 0xFF8E4FD8, x + 2.0, y + 2.0, 2, 2);
            px(t, 0xFFF0C8FF, x, y + 1.0, 1, 1); px(t, 0xFF6FBF4F, x + 1.0, y, 2, 1);
        }
        Objet::Champi => {
            px(t, 0xFF3FB4CC, x, y, 4, 2); px(t, 0xFFBFF6FF, x + 1.0, y, 1, 1);
            px(t, 0xFFEDE6F5, x + 1.0, y + 2.0, 2, 2);
        }
        _ => {}
    }
}

/// Icône d'aliment détourée (pour le menu et le panneau).
pub fn icone_aliment(cle: &str) -> Toile {
    let o = match cle { "pomme" => Objet::Pomme, "baie" => Objet::Baie, _ => Objet::Champi };
    let mut t = Toile::new(8, 8);
    aliment(&mut t, o, 2.0, 3.0);
    t.contour(0xFF1D1520);
    t
}

fn rendre_pose(pose: &Pose, p: &Palette) -> Toile {
    let mut by = pose.by as f32;
    if !pose.air {
        // les pieds touchent le sol : on abaisse le corps si les jambes sont pliées
        let (f, b) = (jambe(20.0, HANCHE, pose.leg_f), jambe(18.0, HANCHE, pose.leg_b));
        by = PIED - f.ay.max(b.ay).max(f.pointe).max(b.pointe).round();
    }
    let u = by + pose.ub as f32;
    let hy = HANCHE + by;

    let mut fond = Toile::new(W, H);
    let main_b = dessiner_bras(&mut fond, p, 18.0, 18.0 + u, pose.arm_b, true);
    objet(&mut fond, p, &main_b, pose.objet_b, pose.obj_b_a, pose);
    dessiner_jambe(&mut fond, p, 18.0, hy, pose.leg_b, true);
    sac(&mut fond, p, u);

    let mut milieu = Toile::new(W, H);
    dessiner_jambe(&mut milieu, p, 20.0, hy, pose.leg_f, false);
    corps(&mut milieu, p, u, by, pose);
    tete(&mut milieu, p, u, pose);
    chapeau(&mut milieu, p, u, pose);

    let mut devant = Toile::new(W, H);
    let main = dessiner_bras(&mut devant, p, 20.0, 18.0 + u, pose.arm_f, false);
    objet(&mut devant, p, &main, pose.objet, pose.obj_a, pose);

    let mut fin = Toile::new(W, H);
    for c in [&mut fond, &mut milieu, &mut devant] {
        c.contour(p.outline);
        fin.coller(c, 0.0, 0.0, 1.0, false, 1.0);
    }

    if let Some(k) = pose.etincelle {
        // étincelles magiques (non détourées)
        let (sx, sy) = (main.x + 4.0, main.y - 1.0);
        let cols = [0xFF6FE7FF, 0xFFE9DCFF, 0xFFB98CFF];
        let c = |i: i32| cols[(i % 3) as usize];
        px(&mut fin, c(k), sx, sy - 1.0 - (k % 2) as f32, 1, 3 + 2 * (k % 2));
        px(&mut fin, c(k), sx - 1.0 - (k % 2) as f32, sy, 3 + 2 * (k % 2), 1);
        px(&mut fin, c(k + 1), sx + 3.0, sy - 4.0 + k as f32, 1, 1);
        px(&mut fin, c(k + 2), sx - 2.0, sy - 5.0 + ((k % 2) * 2) as f32, 1, 1);
    }
    fin
}

// ---------- animations ----------
pub struct Anim {
    pub cle: &'static str,
    pub nom: &'static str,
    pub fps: f32,
    pub boucle: bool,
    pub poses: Vec<Pose>,
}

fn cycle_marche(n: usize, arm_f: Option<(f32, f32)>, objet: Objet) -> Vec<Pose> {
    (0..n).map(|i| {
        let ph = i as f32 / n as f32 * std::f32::consts::TAU;
        let (s, c) = (ph.sin(), ph.cos());
        Pose {
            leg_f: (26.0 * s, 40.0 * c.max(0.0)), leg_b: (-26.0 * s, 40.0 * (-c).max(0.0)),
            arm_f: arm_f.unwrap_or((-24.0 * s, 18.0)), arm_b: (24.0 * s, 18.0), echarpe: (i % 2) as i32, objet,
            ..Default::default()
        }
    }).collect()
}

pub fn anims() -> &'static Vec<Anim> {
    static A: OnceLock<Vec<Anim>> = OnceLock::new();
    A.get_or_init(|| {
        let d = Pose::default;
        let accroupi = Pose { leg_f: (45.0, 90.0), leg_b: (-5.0, 75.0), arm_f: (-30.0, 20.0), arm_b: (-20.0, 20.0), ..d() };
        let demi = Pose { leg_f: (20.0, 40.0), leg_b: (-5.0, 35.0), arm_f: (-15.0, 15.0), arm_b: (5.0, 10.0), ..d() };
        let impulsion = Pose { leg_f: (10.0, 0.0), leg_b: (-20.0, 10.0), arm_f: (160.0, 10.0), arm_b: (140.0, 10.0), chapeau_y: -1, ..d() };
        let groupe = Pose { leg_f: (55.0, 95.0), leg_b: (20.0, 80.0), arm_f: (120.0, 20.0), arm_b: (100.0, 25.0), echarpe: 1, ..d() };
        let pelle = |arm_f, arm_b, obj_a, leg_f, ub, terre| Pose { arm_f, arm_b, objet: Objet::Pelle, obj_a, leg_f, leg_b: (-10.0, 0.0), ub, terre, ..d() };
        let marteau = |arm_f, obj_a, ub| Pose { arm_f, arm_b: (70.0, 20.0), objet: Objet::Marteau, obj_a, leg_f: (12.0, 0.0), leg_b: (-12.0, 0.0), ub, ..d() };
        let grimpe = |leg_f, leg_b, arm_f, arm_b, ub| Pose { air: true, leg_f, leg_b, arm_f, arm_b, ub, ..d() };
        vec![
            Anim { cle: "idle", nom: "Repos (respiration, clignement)", fps: 4.0, boucle: true,
                poses: [0, 0, 1, 1, 1, 1, 0, 0].iter().enumerate()
                    .map(|(i, &ub)| Pose { ub, yeux: if i == 7 { Yeux::Fermes } else { Yeux::Ouverts }, ..d() }).collect() },
            Anim { cle: "walk", nom: "Marche", fps: 10.0, boucle: true, poses: cycle_marche(8, None, Objet::Rien) },
            Anim { cle: "carry", nom: "Marche avec une lettre", fps: 10.0, boucle: true, poses: cycle_marche(8, Some((80.0, 30.0)), Objet::Lettre) },
            Anim { cle: "jump", nom: "Saut", fps: 8.0, boucle: false, poses: vec![accroupi, impulsion, groupe, groupe] },
            Anim { cle: "land", nom: "Réception", fps: 10.0, boucle: false, poses: vec![accroupi, demi, d()] },
            Anim { cle: "fall", nom: "Chute", fps: 8.0, boucle: true, poses: vec![
                Pose { leg_f: (20.0, 30.0), leg_b: (-15.0, 20.0), arm_f: (160.0, -20.0), arm_b: (140.0, 20.0), yeux: Yeux::Haut, bouche: Bouche::O, chapeau_y: -2, echarpe: 1, ..d() },
                Pose { leg_f: (-5.0, 20.0), leg_b: (15.0, 35.0), arm_f: (140.0, 20.0), arm_b: (165.0, -20.0), yeux: Yeux::Haut, bouche: Bouche::O, chapeau_y: -1, ..d() },
            ] },
            Anim { cle: "climb", nom: "Escalade (échelle)", fps: 6.0, boucle: true, poses: vec![
                grimpe((70.0, 100.0), (0.0, 10.0), (150.0, -20.0), (110.0, 20.0), 0),
                grimpe((40.0, 60.0), (20.0, 30.0), (130.0, 0.0), (130.0, 0.0), 1),
                grimpe((0.0, 10.0), (70.0, 100.0), (110.0, 20.0), (150.0, -20.0), 0),
                grimpe((20.0, 30.0), (40.0, 60.0), (130.0, 0.0), (130.0, 0.0), 1),
            ] },
            Anim { cle: "dig", nom: "Creuser", fps: 6.0, boucle: true, poses: vec![
                pelle((60.0, 20.0), (45.0, 25.0), 20.0, (15.0, 10.0), 0, false),
                pelle((40.0, 15.0), (30.0, 20.0), 8.0, (25.0, 35.0), 1, false),
                pelle((70.0, 10.0), (55.0, 20.0), 45.0, (15.0, 10.0), 0, true),
                pelle((110.0, 10.0), (95.0, 15.0), 100.0, (10.0, 0.0), 0, true),
                pelle((150.0, 0.0), (135.0, 10.0), -150.0, (10.0, 0.0), 0, false),
                pelle((90.0, 10.0), (75.0, 15.0), 60.0, (12.0, 5.0), 0, false),
            ] },
            Anim { cle: "build", nom: "Poser une échelle (marteau)", fps: 6.0, boucle: true, poses: vec![
                marteau((150.0, 10.0), 170.0, 0), marteau((115.0, 25.0), 140.0, 0),
                marteau((85.0, 15.0), 95.0, 1), marteau((115.0, 25.0), 140.0, 0),
            ] },
            Anim { cle: "portal", nom: "Ouvrir un portail", fps: 8.0, boucle: true, poses: (0..4).map(|k| Pose {
                arm_f: (95.0 + (k % 2) as f32 * 5.0, -5.0 + k as f32 * 3.0), arm_b: (20.0, 10.0), leg_f: (12.0, 0.0), leg_b: (-12.0, 0.0),
                ub: k % 2, etincelle: Some(k), bouche: Bouche::O, echarpe: k % 2, ..d() }).collect() },
            Anim { cle: "deliver", nom: "Présenter la lettre", fps: 3.0, boucle: true, poses: vec![
                Pose { arm_f: (150.0, 20.0), arm_b: (-10.0, 10.0), objet: Objet::Lettre, bouche: Bouche::Sourire, ..d() },
                Pose { arm_f: (158.0, 12.0), arm_b: (-10.0, 10.0), objet: Objet::Lettre, bouche: Bouche::Sourire, ub: 1, ..d() },
            ] },
            Anim { cle: "wave", nom: "Saluer", fps: 6.0, boucle: true, poses: [-30.0, 0.0, 30.0, 0.0].iter()
                .map(|&e| Pose { arm_f: (165.0, e), arm_b: (5.0, 10.0), bouche: Bouche::Sourire, ..d() }).collect() },
            Anim { cle: "sit", nom: "Assis", fps: 2.0, boucle: true, poses: [0, 0, 1, 1].iter().enumerate().map(|(i, &ub)| Pose {
                air: true, by: 9, ub, leg_f: (88.0, 0.0), leg_b: (80.0, 15.0), arm_f: (-35.0, 0.0), arm_b: (-45.0, 0.0),
                yeux: if i == 3 { Yeux::Fermes } else { Yeux::Ouverts }, ..d() }).collect() },
            Anim { cle: "dangle", nom: "Suspendu (attrapé au col)", fps: 6.0, boucle: true, poses: (0..4).map(|k| {
                let s = [0.0, 1.0, 0.0, -1.0][k];
                Pose { air: true, leg_f: (8.0 + 10.0 * s, 15.0 + 10.0 * s.abs()), leg_b: (-6.0 - 10.0 * s, 20.0), arm_f: (25.0 + 8.0 * s, 10.0),
                    arm_b: (-20.0 - 8.0 * s, 10.0), bouche: Bouche::O, echarpe: (k % 2) as i32, chapeau_y: -1, ..d() }
            }).collect() },
            Anim { cle: "parachute", nom: "Parachute", fps: 3.0, boucle: true, poses: (0..2).map(|k| Pose {
                air: true, leg_f: (10.0 + 6.0 * k as f32, 20.0), leg_b: (-8.0, 25.0 - 8.0 * k as f32), arm_f: (168.0, -8.0), arm_b: (158.0, 6.0),
                bouche: Bouche::Sourire, echarpe: k, ..d() }).collect() },
            manger("eat_pomme", "Manger une pomme", Objet::Pomme),
            manger("eat_baie", "Manger une baie féerique", Objet::Baie),
            manger("eat_champi", "Manger un champignon lumineux", Objet::Champi),
            Anim { cle: "happy", nom: "Content (caresse)", fps: 8.0, boucle: true, poses: (0..4).map(|k| Pose {
                leg_f: if k % 2 == 0 { (25.0, 50.0) } else { (4.0, 0.0) }, leg_b: if k % 2 == 0 { (-5.0, 45.0) } else { (-4.0, 0.0) },
                arm_f: (150.0 + 10.0 * (k % 2) as f32, 10.0), arm_b: (140.0, 10.0), yeux: Yeux::Contents, bouche: Bouche::Sourire,
                echarpe: (k % 2) as i32, ..d() }).collect() },
            Anim { cle: "faible", nom: "Affamé (affaibli)", fps: 1.5, boucle: true, poses: (0..2).map(|k| Pose {
                ub: 2, leg_f: (12.0, 25.0), leg_b: (-6.0, 20.0), arm_f: (-2.0, 0.0), arm_b: (4.0, 0.0),
                yeux: if k == 1 { Yeux::Fermes } else { Yeux::Ouverts }, chapeau_y: 1, ..d() }).collect() },
            Anim { cle: "ko", nom: "KO (couché)", fps: 1.0, boucle: true, poses: vec![
                Pose { air: true, leg_f: (20.0, 0.0), leg_b: (-15.0, 0.0), arm_f: (70.0, 20.0), arm_b: (-60.0, 0.0), yeux: Yeux::Croix, bouche: Bouche::O, ..d() },
            ] },
            Anim { cle: "huff", nom: "Souffler sur l'écran", fps: 3.0, boucle: true, poses: (0..2).map(|k| Pose {
                arm_f: (40.0, 30.0), arm_b: (10.0, 10.0), objet: Objet::Mouchoir, bouche: Bouche::O, ub: k, leg_f: (15.0, 10.0), leg_b: (-8.0, 0.0),
                yeux: if k == 1 { Yeux::Fermes } else { Yeux::Ouverts }, ..d() }).collect() },
            Anim { cle: "wipe", nom: "Nettoyer l'écran (mouchoir)", fps: 7.0, boucle: true,
                poses: [(100.0, 10.0), (122.0, -8.0), (135.0, -25.0), (112.0, -15.0)].iter().enumerate().map(|(k, &arm_f)| Pose {
                    arm_f, arm_b: (20.0 + 6.0 * (k % 2) as f32, 15.0), objet: Objet::Mouchoir, leg_f: (12.0, 0.0), leg_b: (-10.0, 0.0),
                    ub: (k % 2) as i32, echarpe: (k % 2) as i32, ..d() }).collect() },
            Anim { cle: "loupe", nom: "Inspecter à la loupe", fps: 3.0, boucle: true, poses: (0..4).map(|k| Pose {
                arm_f: (75.0 + 6.0 * (k % 2) as f32, 15.0), arm_b: (-20.0, 30.0), objet: Objet::Loupe, obj_a: 95.0 - 8.0 * (k % 2) as f32,
                leg_f: (20.0, 40.0), leg_b: (-5.0, 35.0), ub: 1, yeux: if k == 2 { Yeux::Haut } else { Yeux::Ouverts },
                bouche: if k == 3 { Bouche::O } else { Bouche::Neutre }, ..d() }).collect() },
            Anim { cle: "jumelles", nom: "Espionner aux jumelles", fps: 2.0, boucle: true, poses: (0..4).map(|k| Pose {
                arm_f: (150.0, -60.0), arm_b: (140.0, -50.0), objet: Objet::Jumelles, ub: [0, 1, 0, 0][k],
                leg_f: (8.0, 0.0), leg_b: (-8.0, 0.0), chapeau_y: -[0, 0, 1, 0][k], bouche: if k == 2 { Bouche::O } else { Bouche::Neutre },
                ..d() }).collect() },
            Anim { cle: "trip", nom: "Trébucher", fps: 7.0, boucle: false, poses: vec![
                Pose { leg_f: (30.0, 10.0), leg_b: (-45.0, 70.0), arm_f: (60.0, 30.0), arm_b: (100.0, 20.0), yeux: Yeux::Haut, bouche: Bouche::O, ..d() },
                Pose { air: true, by: 2, leg_f: (-10.0, 30.0), leg_b: (-60.0, 50.0), arm_f: (120.0, 20.0), arm_b: (160.0, -30.0), yeux: Yeux::Haut,
                    bouche: Bouche::O, chapeau_y: -2, echarpe: 1, ..d() },
                Pose { air: true, by: 4, leg_f: (-30.0, 20.0), leg_b: (-70.0, 30.0), arm_f: (95.0, 0.0), arm_b: (110.0, 10.0), yeux: Yeux::Fermes,
                    bouche: Bouche::O, chapeau_y: -3, ..d() },
            ] },
            Anim { cle: "piolet", nom: "Escalade au piolet", fps: 4.0, boucle: true, poses: (0..4).map(|k| {
                // un piolet planté en haut pendant que l'autre remonte ; les pieds cherchent une prise
                let (haut, bas) = (((165.0, -15.0), 165.0), ((120.0, 35.0), 130.0));
                let (f, b) = if k < 2 { (haut, bas) } else { (bas, haut) };
                let (lf, lb) = if k % 2 == 0 { ((70.0, 100.0), (10.0, 20.0)) } else { ((40.0, 60.0), (35.0, 55.0)) };
                Pose { air: true, arm_f: f.0, obj_a: f.1, arm_b: b.0, obj_b_a: b.1, objet: Objet::Piolet, objet_b: Objet::Piolet,
                    leg_f: lf, leg_b: lb, ub: (k % 2) as i32, echarpe: (k % 2) as i32, ..d() }
            }).collect() },
            Anim { cle: "artefact", nom: "Brandir un artefact", fps: 6.0, boucle: true, poses: (0..4).map(|k| Pose {
                arm_f: (170.0 - 4.0 * (k % 2) as f32, 0.0), arm_b: (130.0, 20.0), objet: Objet::Artefact, etincelle: Some(k),
                yeux: Yeux::Contents, bouche: Bouche::Sourire, ub: (k % 2) as i32, echarpe: (k % 2) as i32, ..d() }).collect() },
        ]
    })
}

fn manger(cle: &'static str, nom: &'static str, objet: Objet) -> Anim {
    Anim { cle, nom, fps: 4.0, boucle: true, poses: (0..4).map(|k| Pose {
        arm_f: if k % 2 == 0 { (100.0, 62.0) } else { (85.0, 45.0) }, arm_b: (10.0, 10.0), objet,
        bouche: if k % 2 == 0 { Bouche::O } else { Bouche::Neutre }, yeux: if k == 3 { Yeux::Contents } else { Yeux::Ouverts },
        ub: (k % 2) as i32, ..Pose::default() }).collect() }
}

/// Voile de parachute (rayures rouge et crème) centrée en `x`, bas des suspentes en (`x`, `y`).
pub fn voile(t: &mut Toile, u: i32, x: i32, y: i32, balancement: f32) {
    let (l, h) = (18 * u, 9 * u);
    let haut = y - 16 * u;
    let dx = (balancement * 2.0 * u as f32) as i32;
    for j in 0..h / u {
        let demi = ((1.0 - ((h / u - j) as f32 / (h / u) as f32).powi(2)).sqrt() * l as f32 / u as f32) as i32;
        let yy = haut + j * u;
        t.rect(0xFF1D1520, x + dx - (demi + 1) * u, yy, (2 * demi + 2) * u, u);
        for i in -demi..demi {
            let bande = ((i + demi) / 3) % 2 == 0;
            t.rect(if bande { 0xFFD83A2E } else { 0xFFF4E9CF }, x + dx + i * u, yy, u, u);
        }
    }
    t.rect(0xFF1D1520, x + dx - (l / u + 1) * u, haut + h, (2 * (l / u) + 2) * u, u);
    for (sx, ex) in [(-(l / u) + 1, -2), (-(l / u) / 2, -1), ((l / u) / 2, 1), ((l / u) - 1, 2)] {
        let (x0, y0, x1, y1) = (x + dx + sx * u, haut + h + u, x + ex * u, y);
        let n = ((y1 - y0) / u).max(1);
        for k in 0..=n { t.rect(0xFF3A2414, x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n, u.max(1) / 2 + 1, u.max(1) / 2 + 1); }
    }
}

/// Pierre tombale moussue avec « R.I.P. ».
pub fn tombe(t: &mut Toile, u: i32, x: i32, y: i32) {
    let (w, h) = (12, 15);
    let (g, haut) = (x - w / 2 * u, y - h * u);
    t.rect(0xFF1D1520, g - u, haut + u, (w + 2) * u, h * u);
    t.rect(0xFF1D1520, g, haut, w * u, u);
    t.rect(0xFF8A84A8, g, haut + u, w * u, (h - 1) * u);
    t.rect(0xFFB0AACB, g, haut + u, u, (h - 1) * u);
    t.rect(0xFF5E5880, g + (w - 1) * u, haut + u, u, (h - 1) * u);
    for (i, c) in [(2, 0xFF6FBF4F), (3, 0xFF3F6B3A), (9, 0xFF6FBF4F)] { t.rect(c, g + i * u, haut + u, u, u); }
    // croix
    t.rect(0xFF4A4466, x - u / 2 - u / 2, haut + 3 * u, u * 2, 7 * u);
    t.rect(0xFF4A4466, x - 3 * u + u / 2, haut + 5 * u, 5 * u, u * 2);
    t.rect(0xFF3F6B3A, g - u, y - u, (w + 2) * u, u);
    t.rect(0xFF6FBF4F, g + 2 * u, y - 2 * u, u, u);
    t.rect(0xFFFF7AD9, g + (w - 3) * u, y - 2 * u, u, u);
}

/// Petit cœur rose de 5 × 4 (u pixels par point).
pub fn coeur(t: &mut Toile, u: i32, x: i32, y: i32, alpha: f32) {
    let c = (((alpha.clamp(0.0, 1.0) * 255.0) as u32) << 24) | 0x00FF5C8A;
    for (dx, dy, w) in [(0, 0, 2), (3, 0, 2), (0, 1, 5), (1, 2, 3), (2, 3, 1)] { t.rect(c, x + dx * u, y + dy * u, w * u, u); }
}

/// Étoile jaune d'étourdissement.
pub fn etoile(t: &mut Toile, u: i32, x: i32, y: i32) {
    t.rect(0xFFFFE066, x, y - u, u, 3 * u);
    t.rect(0xFFFFE066, x - u, y, 3 * u, u);
}

/// Buée et traces de doigts sur la vitre, centrées en (`x`, `y`) ; `p` = opacité (1 = sale, 0 = propre).
pub fn buee(t: &mut Toile, u: i32, x: i32, y: i32, p: f32) {
    let a = (p.clamp(0.0, 1.0) * 150.0) as u32;
    if a == 0 { return; }
    let voile = (a << 24) | 0x00D8E4EC;
    for (dx, dy, w, h) in [(-5, -3, 10, 6), (-7, -1, 14, 3), (-3, -5, 6, 10), (-6, -2, 12, 5)] {
        t.rect(voile, x + dx * u, y + dy * u, w * u, h * u);
    }
    // deux empreintes de doigts, plus marquées
    let trace = (((a * 3 / 4) as u32) << 24) | 0x00A8B4C0;
    for (cx, cy) in [(-3, -1), (3, 1)] {
        for (dx, dy, w) in [(0, 0, 3), (-1, 1, 5), (-1, 2, 5), (0, 3, 3)] { t.rect(trace, x + (cx + dx) * u, y + (cy + dy - 2) * u, w * u, u / 2 + 1); }
    }
}

/// Petit caillou posé sur le sol `y`, centré en `x`.
pub fn caillou(t: &mut Toile, u: i32, x: i32, y: i32) {
    t.rect(NOIR, x - 3 * u, y - 3 * u, 6 * u, 3 * u);
    t.rect(NOIR, x - 2 * u, y - 4 * u, 4 * u, u);
    t.rect(0xFF8A8494, x - 2 * u, y - 3 * u, 4 * u, 2 * u);
    t.rect(0xFFB5B0BF, x - u, y - 3 * u, u, u);
    t.rect(0xFF5E5868, x - 2 * u, y - u, 4 * u, u);
}

/// Éclat de lumière magique qui s'étend autour de (`x`, `y`) ; `p` = temps écoulé (s).
pub fn eclat(t: &mut Toile, u: i32, x: i32, y: i32, p: f32) {
    let r = 2.0 + p.min(1.2) * 14.0;
    let cols = [0xFF6FE7FF, 0xFFE9DCFF, 0xFFFFE066, 0xFFB98CFF];
    for i in 0..10 {
        let ang = i as f32 * std::f32::consts::TAU / 10.0 + p * 2.0;
        let rr = r * if i % 2 == 0 { 1.0 } else { 0.7 };
        let (sx, sy) = (x + (ang.cos() * rr * u as f32) as i32, y + (ang.sin() * rr * u as f32) as i32);
        let c = cols[i % 4];
        t.rect(c, sx, sy - u, u, 3 * u);
        t.rect(c, sx - u, sy, 3 * u, u);
    }
    if p < 0.4 { t.rect(0xFFFFFFFF, x - 2 * u, y - 2 * u, 4 * u, 4 * u); }
}

pub fn anim(cle: &str) -> &'static Anim {
    let a = anims();
    a.iter().find(|x| x.cle == cle).unwrap_or(&a[0])
}

pub fn index_image(cle: &str, t: f32) -> usize {
    let a = anim(cle);
    let i = (t.max(0.0) * a.fps).floor() as usize;
    if a.boucle { i % a.poses.len() } else { i.min(a.poses.len() - 1) }
}

/// Cache des images déjà rendues.
#[derive(Default)]
pub struct Sprites {
    cache: HashMap<(String, &'static str, usize), Toile>,
}

impl Sprites {
    pub fn image_n(&mut self, look: &str, cle: &str, i: usize) -> &Toile {
        let a = anim(cle);
        let i = i.min(a.poses.len() - 1);
        self.cache.entry((look.to_string(), a.cle, i)).or_insert_with(|| rendre_pose(&a.poses[i], &palette(look)))
    }
    pub fn image(&mut self, look: &str, cle: &str, t: f32) -> &Toile {
        let i = index_image(cle, t);
        self.image_n(look, cle, i)
    }
}

// ---------- décors (u = taille d'un « gros pixel » à l'écran) ----------
const NOIR: u32 = 0xFF1D1520;

/// Échelle centrée en `x`, de `y_haut` à `y_bas` ; `p` = avancement de la pose (0 → 1).
pub fn echelle(t: &mut Toile, u: i32, x: i32, y_haut: i32, y_bas: i32, p: f32, vers_bas: bool) {
    let vis = ((y_bas - y_haut) as f32 * p.min(1.0)) as i32;
    if vis < 2 { return; }
    let y0 = if vers_bas { y_haut } else { y_bas - vis };
    let y1 = y0 + vis;
    let (g, d) = (x - 8 * u, x + 6 * u);
    let mut y = y_bas - 5 * u;
    while y > y_haut + u {
        // barreaux, accrochés au bas de l'échelle
        if y >= y0 && y <= y1 - u { t.rect(NOIR, g + u, y - u, d - g, 3 * u); t.rect(0xFF9A6A35, g + u, y, d - g, u); }
        y -= 7 * u;
    }
    for xx in [g, d] {
        t.rect(NOIR, xx - u, y0 - u, 4 * u, vis + 2 * u);
        t.rect(0xFF9A6A35, xx, y0, 2 * u, vis);
        t.rect(0xFFC08A4E, xx, y0, u, vis);
    }
}

/// Trou dans le sol, centré en `x`, sur la ligne de sol `y`, avec son tas de terre.
pub fn trou(t: &mut Toile, u: i32, x: i32, y: i32, p: f32) {
    let w = ((10.0 * p.min(1.0)).round() as i32).max(1);
    for (i, hw) in [w - 4, w - 2, w - 1, w, w - 1, w - 3].into_iter().enumerate() {
        if hw <= 0 { continue; }
        let yy = y + (i as i32 - 4) * u;
        t.rect(NOIR, x - (hw + 1) * u, yy, (2 * hw + 2) * u, u);
        t.rect(if i < 2 { 0xFF4A2F1D } else { 0xFF0B0709 }, x - hw * u, yy, 2 * hw * u, u);
    }
    let tx = x + (w + 5) * u; // tas de terre
    for (hw, i) in [(1, 0), (3, 1), (5, 2)] {
        let n = (hw as f32 * p.min(1.0)).round() as i32;
        if n == 0 { continue; }
        let yy = y + (i - 4) * u;
        t.rect(NOIR, tx - (n + 1) * u, yy - u, (2 * n + 2) * u, 2 * u);
        t.rect(if i > 0 { 0xFF6B4A2B } else { 0xFF8A6238 }, tx - n * u, yy, 2 * n * u, u);
    }
}

/// Portail ovale tourbillonnant posé sur le sol `y`, centré en `x`.
pub fn portail(t: &mut Toile, u: i32, x: i32, y: i32, p: f32, temps: f32) {
    let p = p.clamp(0.0, 1.0);
    if p <= 0.03 { return; }
    let (rx, ry) = (10.0 * p, 18.0 * p);
    let cy = y - 19 * u;
    let cols = [0xFF24104A, 0xFF5B2BB5, 0xFF9B6BFF];
    let (nx, ny) = (rx.ceil() as i32, ry.ceil() as i32);
    for j in -ny..=ny {
        for i in -nx..=nx {
            let d = ((i as f32 / rx).powi(2) + (j as f32 / ry).powi(2)).sqrt();
            if d > 1.0 { continue; }
            let ang = (j as f32).atan2(i as f32);
            let col = if d > 0.84 {
                if ((temps * 8.0 + ang * 3.0).floor() as i32) & 1 == 1 { 0xFFE9DCFF } else { 0xFFB98CFF }
            } else if d < 0.22 {
                0xFF0B0420
            } else {
                let k = (d * 5.0 - temps * 3.0 + ang / std::f32::consts::PI * 2.0).floor() as i32;
                cols[k.rem_euclid(3) as usize]
            };
            t.rect(col, x + i * u, cy + j * u, u, u);
        }
    }
}

/// Planche (storyboard) de toutes les animations, pour l'export PNG.
pub fn planche(look: &str, k: i32) -> Toile {
    let mut s = Sprites::default();
    let a = anims();
    let maxn = a.iter().map(|x| x.poses.len()).max().unwrap_or(1) as i32;
    let mut t = Toile::new(maxn * W * k, a.len() as i32 * H * k);
    t.rect(0xFFE8E2D0, 0, 0, t.w, t.h);
    for (r, an) in a.iter().enumerate() {
        for i in 0..an.poses.len() {
            let fond = if (r + i) % 2 == 0 { 0xFFDDD5BE } else { 0xFFE8E2D0 };
            t.rect(fond, i as i32 * W * k, r as i32 * H * k, W * k, H * k);
            let img = s.image_n(look, an.cle, i).clone();
            t.coller(&img, (i as i32 * W * k) as f32, (r as i32 * H * k) as f32, k as f32, false, 1.0);
        }
    }
    t
}
