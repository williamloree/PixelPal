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
pub enum Yeux { Ouverts, Fermes, Haut }
#[derive(Clone, Copy, PartialEq)]
pub enum Bouche { Neutre, O, Sourire }
#[derive(Clone, Copy, PartialEq)]
pub enum Objet { Rien, Lettre, Pelle, Marteau }

#[derive(Clone, Copy)]
pub struct Pose {
    leg_f: (f32, f32), leg_b: (f32, f32), arm_f: (f32, f32), arm_b: (f32, f32),
    ub: i32, by: i32, air: bool, yeux: Yeux, bouche: Bouche, chapeau_y: i32, echarpe: i32,
    objet: Objet, obj_a: f32, terre: bool, etincelle: Option<i32>,
}

impl Default for Pose {
    fn default() -> Self {
        Pose { leg_f: (4.0, 0.0), leg_b: (-4.0, 0.0), arm_f: (-8.0, 14.0), arm_b: (10.0, 10.0), ub: 0, by: 0, air: false,
            yeux: Yeux::Ouverts, bouche: Bouche::Neutre, chapeau_y: 0, echarpe: 0, objet: Objet::Rien, obj_a: 0.0, terre: false, etincelle: None }
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

fn objet(t: &mut Toile, p: &Palette, m: &Main, pose: &Pose) {
    match pose.objet {
        Objet::Lettre => {
            px(t, p.paper, m.x - 2.0, m.y - 3.0, 6, 4); px(t, p.paper_shade, m.x - 2.0, m.y, 6, 1);
            px(t, p.paper_shade, m.x - 1.0, m.y - 2.0, 1, 1); px(t, p.paper_shade, m.x + 2.0, m.y - 2.0, 1, 1);
            px(t, p.seal, m.x, m.y - 1.0, 2, 1);
        }
        Objet::Pelle => {
            let (a, b) = dir(pose.obj_a);
            let (cx, cy) = (m.x + 0.5, m.y + 0.5);
            trait_(t, p.wood, cx - a * 4.0, cy - b * 4.0, cx + a * 7.0, cy + b * 7.0, 1);
            let (bx, by) = (cx + a * 8.0, cy + b * 8.0);
            px(t, p.metal, bx - 1.0, by - 1.0, 3, 3); px(t, p.metal_shade, bx - 1.0, by + 1.0, 3, 1);
            if pose.terre { px(t, p.dirt, bx - 1.0, by - 2.0, 3, 1); }
        }
        Objet::Marteau => {
            let (a, b) = dir(pose.obj_a);
            trait_(t, p.wood, m.x + 0.5, m.y + 0.5, m.x + 0.5 + a * 5.0, m.y + 0.5 + b * 5.0, 1);
            px(t, p.metal, m.x + a * 5.0 - 1.0, m.y + b * 5.0 - 1.0, 3, 2);
        }
        Objet::Rien => {}
    }
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
    dessiner_bras(&mut fond, p, 18.0, 18.0 + u, pose.arm_b, true);
    dessiner_jambe(&mut fond, p, 18.0, hy, pose.leg_b, true);
    sac(&mut fond, p, u);

    let mut milieu = Toile::new(W, H);
    dessiner_jambe(&mut milieu, p, 20.0, hy, pose.leg_f, false);
    corps(&mut milieu, p, u, by, pose);
    tete(&mut milieu, p, u, pose);
    chapeau(&mut milieu, p, u, pose);

    let mut devant = Toile::new(W, H);
    let main = dessiner_bras(&mut devant, p, 20.0, 18.0 + u, pose.arm_f, false);
    objet(&mut devant, p, &main, pose);

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
        ]
    })
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
