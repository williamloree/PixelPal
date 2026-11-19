//! Décor du panneau : police pixel 5 × 7, scène animée (temple au fond d'un jardin féerique,
//! le personnage court sur le chemin de pierre façon Temple Run), texture de pierre et icônes.

use crate::sprites::Sprites;
use crate::toile::Toile;
use std::f32::consts::PI;

// ---------- couleurs ----------
pub const OR: u32 = 0xFFF2C14E;
pub const OR_CLAIR: u32 = 0xFFFFE9A8;
pub const CREME: u32 = 0xFFEFE6CF;
pub const LAVANDE: u32 = 0xFFB9A8E0;
pub const NUIT: u32 = 0xFF1D1520;
pub const FEUILLE: u32 = 0xFF6FBF4F;
pub const MOUSSE: u32 = 0xFF3F6B3A;

fn mel(a: u32, b: u32, k: f32) -> u32 {
    let k = k.clamp(0.0, 1.0);
    let c = |s: u32| ((((a >> s) & 255) as f32 * (1.0 - k) + ((b >> s) & 255) as f32 * k).round() as u32) << s;
    0xFF00_0000 | c(16) | c(8) | c(0)
}
fn alpha(c: u32, a: f32) -> u32 { (c & 0x00FF_FFFF) | (((a.clamp(0.0, 1.0) * 255.0) as u32) << 24) }
fn h(n: i64) -> f32 {
    let mut x = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 32;
    (x & 0xFF_FFFF) as f32 / 16_777_216.0
}
fn p(t: &mut Toile, c: u32, x: i32, y: i32) { t.rect(c, x, y, 1, 1); }
fn halo(t: &mut Toile, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
    let n = r.ceil() as i32;
    for j in -n..=n {
        for i in -n..=n {
            let d = ((i * i + j * j) as f32).sqrt() / r;
            if d < 1.0 { p(t, alpha(c, a * (1.0 - d) * (1.0 - d)), cx as i32 + i, cy as i32 + j); }
        }
    }
}
fn disque(t: &mut Toile, cx: i32, cy: i32, r: f32, c: u32) {
    let n = r.ceil() as i32;
    for j in -n..=n { for i in -n..=n { if ((i * i + j * j) as f32) <= r * r { p(t, c, cx + i, cy + j); } } }
}

// ---------- police pixel 5 × 7 (majuscules, chiffres, accents) ----------
fn glyphe(c: char) -> Option<&'static [&'static str; 7]> {
    Some(match c {
        'A' => &[".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'B' => &["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
        'C' => &[".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
        'D' => &["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
        'E' => &["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
        'F' => &["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
        'G' => &[".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###."],
        'H' => &["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'I' => &["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"],
        'J' => &["..###", "...#.", "...#.", "...#.", "#..#.", "#..#.", ".##.."],
        'K' => &["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
        'L' => &["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
        'M' => &["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'N' => &["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#"],
        'O' => &[".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'P' => &["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
        'Q' => &[".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
        'R' => &["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'S' => &[".####", "#....", "#....", ".###.", "....#", "....#", "####."],
        'T' => &["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        'U' => &["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => &["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'W' => &["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#"],
        'X' => &["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
        'Y' => &["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
        'Z' => &["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
        '0' => &[".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."],
        '1' => &[".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"],
        '2' => &[".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
        '3' => &["####.", "....#", "....#", ".###.", "....#", "....#", "####."],
        '4' => &["#..#.", "#..#.", "#..#.", "#####", "...#.", "...#.", "...#."],
        '5' => &["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
        '6' => &[".###.", "#....", "#....", "####.", "#...#", "#...#", ".###."],
        '7' => &["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
        '8' => &[".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
        '9' => &[".###.", "#...#", "#...#", ".####", "....#", "....#", ".###."],
        ' ' => &["...", "...", "...", "...", "...", "...", "..."],
        '.' => &[".", ".", ".", ".", ".", ".", "#"],
        ',' => &["..", "..", "..", "..", "..", ".#", "#."],
        ':' => &[".", "#", ".", ".", ".", "#", "."],
        '!' => &["#", "#", "#", "#", "#", ".", "#"],
        '?' => &[".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
        '\'' | '’' => &["#", "#", ".", ".", ".", ".", "."],
        '-' => &["...", "...", "...", "###", "...", "...", "..."],
        '·' => &[".", ".", ".", "#", ".", ".", "."],
        '/' => &["....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#...."],
        '(' => &[".#", "#.", "#.", "#.", "#.", "#.", ".#"],
        ')' => &["#.", ".#", ".#", ".#", ".#", ".#", "#."],
        _ => return None,
    })
}

#[derive(Clone, Copy)]
enum Accent { Aigu, Grave, Circ, Trema, Cedille }

fn decomposer(c: char) -> (char, Option<Accent>) {
    let c = c.to_uppercase().next().unwrap_or(c);
    match c {
        'É' => ('E', Some(Accent::Aigu)), 'È' => ('E', Some(Accent::Grave)), 'Ê' => ('E', Some(Accent::Circ)), 'Ë' => ('E', Some(Accent::Trema)),
        'À' => ('A', Some(Accent::Grave)), 'Â' => ('A', Some(Accent::Circ)), 'Î' => ('I', Some(Accent::Circ)), 'Ï' => ('I', Some(Accent::Trema)),
        'Ô' => ('O', Some(Accent::Circ)), 'Û' => ('U', Some(Accent::Circ)), 'Ù' => ('U', Some(Accent::Grave)), 'Ç' => ('C', Some(Accent::Cedille)),
        _ => (c, None),
    }
}

/// Largeur en pixels d'un texte dans la police pixel.
pub fn largeur(s: &str) -> i32 {
    s.chars().map(|c| glyphe(decomposer(c).0).map(|g| g[0].len() as i32 + 1).unwrap_or(4)).sum::<i32>() - 1
}

/// Écrit un texte ; `vague` fait onduler les lettres (temps). Renvoie la largeur.
pub fn ecrire(t: &mut Toile, s: &str, x: i32, y: i32, c: u32, vague: Option<f32>) -> i32 {
    let mut cx = x;
    for (i, ch) in s.chars().enumerate() {
        let (base, acc) = decomposer(ch);
        let Some(g) = glyphe(base) else { cx += 4; continue };
        let w = g[0].len() as i32;
        let dy = vague.map(|tm| ((tm * 3.0 + i as f32 * 0.6).sin() * 1.5).round() as i32).unwrap_or(0);
        for (r, ligne) in g.iter().enumerate() {
            for (k, b) in ligne.bytes().enumerate() { if b == b'#' { p(t, c, cx + k as i32, y + dy + r as i32); } }
        }
        let m = cx + w / 2;
        let y0 = y + dy;
        match acc {
            Some(Accent::Aigu) => { p(t, c, m + 1, y0 - 2); p(t, c, m, y0 - 1); }
            Some(Accent::Grave) => { p(t, c, m - 1, y0 - 2); p(t, c, m, y0 - 1); }
            Some(Accent::Circ) => { p(t, c, m, y0 - 2); p(t, c, m - 1, y0 - 1); p(t, c, m + 1, y0 - 1); }
            Some(Accent::Trema) => { p(t, c, m - 1, y0 - 1); p(t, c, m + 1, y0 - 1); }
            Some(Accent::Cedille) => { p(t, c, m, y0 + 7); p(t, c, m - 1, y0 + 8); }
            None => {}
        }
        cx += w + 1;
    }
    cx - x - 1
}

/// Texte avec contour sombre, sur une toile ajustée (marge pour accents et contour).
pub fn etiquette(s: &str, c: u32, contour: u32) -> Toile {
    let mut t = Toile::new(largeur(s) + 4, 13);
    ecrire(&mut t, s, 2, 3, c, None);
    t.contour(contour);
    t
}

// ---------- icônes 9 × 9 ----------
pub fn icone(nom: &str, c: u32) -> Toile {
    let motif: [&str; 9] = match nom {
        "lettre" => [".........", "#########", "##.....##", "#.#...#.#", "#..#.#..#", "#...#...#", "#.......#", "#########", "........."],
        "groupe" => ["..##..##.", ".####.###", ".####.###", "..##..##.", ".........", "####.####", "####.####", "####.####", "........."],
        "lien" => ["....###..", "...#...#.", "...#...#.", "....#.#..", "...#.#...", "..#.#....", ".#...#...", ".#...#...", "..###...."],
        "ecran" => ["#########", "#.......#", "#.......#", "#.......#", "#.......#", "#########", "....#....", "...###...", "........."],
        "coeur" => [".........", ".##...##.", "####.####", "#########", "#########", ".#######.", "..#####..", "...###...", "....#...."],
        "chapeau" => [".........", "...###...", "..#####..", "..#####..", "..#...#..", "#########", ".#######.", ".........", "........."],
        _ => ["........."; 9],
    };
    let mut t = Toile::new(11, 11);
    for (y, l) in motif.iter().enumerate() {
        for (x, b) in l.bytes().enumerate() { if b == b'#' { p(&mut t, c, x as i32 + 1, y as i32 + 1); } }
    }
    t.contour(NUIT);
    t
}

/// Dalle de pierre sombre et moussue (motif répétable 32 × 32).
pub fn dalle() -> Toile {
    let mut t = Toile::new(32, 32);
    t.rect(0xFF141126, 0, 0, 32, 32);
    for rang in 0..4 {
        let dec = if rang % 2 == 0 { 0 } else { 8 };
        for b in -1..2 {
            let x = b * 16 + dec;
            let teinte = mel(0xFF1E1A34, 0xFF26213F, h((rang * 7 + b + 3) as i64));
            t.rect(teinte, x + 1, rang * 8 + 1, 15, 7);
            t.rect(mel(teinte, 0xFF3A3458, 0.35), x + 1, rang * 8 + 1, 15, 1);
            t.rect(mel(teinte, 0xFF0D0B18, 0.4), x + 1, rang * 8 + 7, 15, 1);
        }
    }
    for i in 0..18 {
        let (x, y) = ((h(i) * 32.0) as i32, (h(i + 50) * 32.0) as i32);
        p(&mut t, if i % 3 == 0 { 0xFF2C4A2E } else { 0xFF22382A }, x, y);
    }
    t
}

// ---------- la scène ----------
// Scène calme : il attend devant sa maisonnette, un château féerique sur la colline au fond.
// Peu de mouvement : étoiles qui scintillent, fumée de la cheminée, quelques lucioles, une fée qui flâne.
pub const SW: i32 = 200;
pub const SH: i32 = 80;
const SOL: i32 = 66;
const PERSO_X: f32 = 108.0;

fn ciel(t: &mut Toile, temps: f32) {
    let degrade = [(0.0, 0xFF120A2A), (0.45, 0xFF2A1A5E), (0.75, 0xFF4B2F7E), (1.0, 0xFF7A4A8E)];
    for y in 0..SH {
        let k = y as f32 / SH as f32;
        let i = degrade.iter().rposition(|d| d.0 <= k).unwrap_or(0).min(degrade.len() - 2);
        let (a, b) = (degrade[i], degrade[i + 1]);
        t.rect(mel(a.1, b.1, (k - a.0) / (b.0 - a.0)), 0, y, SW, 1);
    }
    for i in 0..45 {
        let (x, y) = ((h(i) * SW as f32) as i32, (h(i + 100) * SH as f32 * 0.5) as i32);
        let sc = 0.55 + 0.45 * ((temps * (0.3 + h(i + 200) * 0.6)) + i as f32).sin();
        let c = if i % 4 == 0 { 0xFFFFE9A8 } else { 0xFFE9DCFF };
        p(t, alpha(c, sc), x, y);
    }
    let (lx, ly) = (128, 13);
    halo(t, lx as f32, ly as f32, 16.0, 0xFFE9DCFF, 0.3);
    disque(t, lx, ly, 7.5, 0xFFF6EBC8);
    for (x, y) in [(-3, -2), (2, 2), (-1, 3), (3, -3)] { p(t, 0xFFE2D3A8, lx + x, ly + y); }
    disque(t, lx - 3, ly - 2, 1.0, 0xFFE2D3A8);
}

fn collines(t: &mut Toile) {
    for x in 0..SW {
        let xf = x as f32;
        let hy = 50.0 + (xf * 0.03).sin() * 4.0 + (xf * 0.071 + 1.0).sin() * 2.5;
        t.rect(0xFF261C4A, x, hy as i32, 1, SH);
        // colline du château, à droite
        let c = 47.0 - (1.0 - ((xf - 160.0) / 42.0).powi(2)).max(0.0) * 9.0;
        t.rect(0xFF2B2054, x, c.min(hy) as i32, 1, SH);
        let hy2 = 57.0 + (xf * 0.05 + 2.0).sin() * 2.0;
        t.rect(0xFF1E1840, x, hy2 as i32, 1, SH);
    }
}

fn chateau(t: &mut Toile, temps: f32) {
    let (pierre, ombre, toit) = (0xFF4A4466, 0xFF35304F, 0xFF6B3FA0);
    let base = 39;
    t.rect(pierre, 152, 28, 16, base - 28);
    t.rect(ombre, 165, 28, 3, base - 28);
    for x in (152..168).step_by(3) { t.rect(pierre, x, 26, 2, 2); }
    for (tx, haut) in [(146, 24), (168, 22)] {
        t.rect(pierre, tx, haut, 6, base - haut);
        t.rect(ombre, tx + 4, haut, 2, base - haut);
        for j in 0..7 { t.rect(toit, tx + 3 - j / 2, haut - 7 + j, 1 + j / 2 * 2, 1); }
        p(t, OR, tx + 2, haut - 9);
        p(t, OR, tx + 3, haut - 9);
    }
    // fenêtres allumées qui scintillent doucement
    for (i, (x, y)) in [(148, 30), (170, 28), (157, 32), (162, 32), (159, 28)].into_iter().enumerate() {
        let l = 0.75 + 0.25 * (temps * 0.8 + i as f32 * 1.7).sin();
        p(t, mel(0xFF8A5A14, OR, l), x, y);
    }
    t.rect(0xFF2A2540, 158, base - 4, 4, 4);
}

fn arbre(t: &mut Toile, x: i32) {
    t.rect(0xFF1A2230, x + 5, 40, 3, SOL - 40);
    disque(t, x + 6, 34, 9.0, 0xFF1F3B2E);
    disque(t, x + 2, 39, 6.0, 0xFF1F3B2E);
    disque(t, x + 11, 39, 5.0, 0xFF1F3B2E);
    for a in 0..12 { let an = a as f32 / 12.0 * PI; p(t, 0xFF2D5A40, x + 6 + (an.cos() * 9.0) as i32, 34 - (an.sin() * 9.0) as i32); }
}

fn maison(t: &mut Toile, temps: f32) {
    let (g, d, haut_mur) = (46, 82, 47);
    // murs à colombages
    t.rect(NUIT, g - 1, haut_mur - 1, d - g + 2, SOL - haut_mur + 1);
    t.rect(0xFFD9CDB0, g, haut_mur, d - g, SOL - haut_mur);
    t.rect(0xFFBFB190, g, SOL - 3, d - g, 3);
    for x in [g, g + 12, g + 24, d - 2] { t.rect(0xFF5C3A1F, x, haut_mur, 2, SOL - haut_mur); }
    t.rect(0xFF5C3A1F, g, haut_mur, d - g, 2);
    // toit de tuiles
    let sommet = 31;
    for j in 0..=(haut_mur - sommet) {
        let demi = 4 + j * 22 / (haut_mur - sommet);
        let y = sommet + j;
        let c = if j % 3 == 0 { 0xFF6A2E24 } else { 0xFF8E3B2E };
        t.rect(NUIT, 64 - demi - 1, y, 2 * demi + 2, 1);
        t.rect(c, 64 - demi, y, 2 * demi, 1);
    }
    for (x, y) in [(56, 38), (71, 41), (63, 34)] { t.rect(0xFF3F6B3A, x, y, 3, 1); }
    // cheminée et fumée lente
    t.rect(NUIT, 72, 30, 6, 8);
    t.rect(0xFF6B6585, 73, 30, 4, 8);
    for i in 0..4 {
        let ph = (temps * 0.12 + i as f32 / 4.0).rem_euclid(1.0);
        let (x, y) = (75.0 + (ph * 5.0 + i as f32).sin() * 2.0 + ph * 7.0, 28.0 - ph * 22.0);
        let r = 1.5 + ph * 2.5;
        halo(t, x, y, r + 1.5, 0xFFB9A8E0, 0.45 * (1.0 - ph));
    }
    // porte et fenêtre allumée
    t.rect(NUIT, 51, 53, 8, SOL - 53);
    t.rect(0xFF5C3A1F, 52, 54, 6, SOL - 54);
    t.rect(0xFF7A4E2B, 52, 54, 2, SOL - 54);
    p(t, OR, 56, 60);
    let l = 0.85 + 0.15 * (temps * 0.7).sin();
    halo(t, 70.0, 55.0, 9.0, 0xFFFFD36B, 0.25 * l);
    t.rect(NUIT, 65, 51, 10, 9);
    t.rect(mel(0xFFE0A030, 0xFFFFE9A8, l), 66, 52, 8, 7);
    t.rect(0xFF5C3A1F, 69, 52, 1, 7);
    t.rect(0xFF5C3A1F, 66, 55, 8, 1);
    // lanterne
    t.rect(0xFF3A2414, 90, 51, 1, SOL - 51);
    t.rect(0xFF3A2414, 88, 50, 5, 1);
    halo(t, 90.5, 53.0, 6.0, 0xFFFFB347, 0.35 * l);
    t.rect(0xFFFFD36B, 89, 51, 3, 3);
}

fn sol(t: &mut Toile, temps: f32) {
    t.rect(0xFF2C3A2A, 0, SOL, SW, SH - SOL);
    t.rect(MOUSSE, 0, SOL, SW, 1);
    // chemin de dalles jusqu'à la porte
    for (i, x) in [54, 62, 72, 84, 98, 112].into_iter().enumerate() {
        let y = SOL + 2 + (i as i32 % 2) * 3;
        t.rect(0xFF2A2540, x - 1, y - 1, 8, 4);
        t.rect(0xFF57527A, x, y, 6, 2);
    }
    for k in 0..50i64 {
        let x = (k * 4 + (h(k) * 3.0) as i64) as i32;
        let hh = (h(k + 7) * 3.0) as i32;
        for j in 0..=hh { p(t, if j == hh { 0xFF5C9A45 } else { MOUSSE }, x + (j % 2), SOL - j); }
        if h(k + 900) > 0.85 {
            let c = if k % 3 == 0 { 0xFFFF7AD9 } else if k % 3 == 1 { 0xFF6FE7FF } else { 0xFFFFE066 };
            let l = 0.7 + 0.3 * (temps * 0.9 + k as f32).sin();
            halo(t, x as f32, (SOL - 4) as f32, 3.0, c, 0.4 * l);
            p(t, MOUSSE, x, SOL - 1); p(t, MOUSSE, x, SOL - 2);
            p(t, c, x, SOL - 3); p(t, c, x - 1, SOL - 3); p(t, c, x + 1, SOL - 3);
        }
    }
}

fn fee(t: &mut Toile, temps: f32) {
    let (x, y) = (PERSO_X + 22.0 + (temps * 0.5).sin() * 14.0, 36.0 + (temps * 0.8).sin() * 4.0);
    halo(t, x, y, 6.0, 0xFFFF9AE3, 0.5 + 0.1 * (temps * 2.0).sin());
    let aile = ((temps * 10.0) as i32) % 2;
    let ca = alpha(0xFFBFF6FF, 0.85);
    p(t, ca, x as i32 - 1, y as i32 - 1 - aile); p(t, ca, x as i32 + 2, y as i32 - 1 - aile);
    t.rect(0xFFFFFFFF, x as i32, y as i32, 2, 2);
}

fn lucioles(t: &mut Toile, temps: f32) {
    for i in 0..6 {
        let x = h(i + 600) * SW as f32 + (temps * 0.15 + i as f32).sin() * 12.0;
        let y = 40.0 + h(i + 700) * 20.0 + (temps * 0.3 + i as f32 * 2.0).sin() * 4.0;
        let l = (temps * 0.8 + i as f32 * 1.7).sin().max(0.0);
        if l < 0.05 { continue; }
        halo(t, x, y, 3.0, 0xFFD8FF7A, 0.5 * l);
        p(t, alpha(0xFFF4FFC8, l), x as i32, y as i32);
    }
}

/// Dessine une image de la scène (SW × SH pixels).
pub fn scene(t: &mut Toile, temps: f32, sprites: &mut Sprites, look: &str, version: &str) {
    ciel(t, temps);
    collines(t);
    chateau(t, temps);
    arbre(t, 4);
    arbre(t, 182);
    maison(t, temps);
    sol(t, temps);
    // il attend devant chez lui ; de temps en temps il s'assoit
    let assis = (temps % 40.0) > 26.0;
    let img = if assis { sprites.image(look, "sit", temps) } else { sprites.image(look, "idle", temps) }.clone();
    t.rect(alpha(0xFF000000, 0.3), PERSO_X as i32 - 5, SOL, 10, 1);
    t.coller(&img, PERSO_X - crate::sprites::CX as f32, SOL as f32 - crate::sprites::BAS as f32, 1.0, true, 1.0);
    fee(t, temps);
    lucioles(t, temps);
    // titre doré, fixe
    let mut titre = Toile::new(60, 14);
    ecrire(&mut titre, "PIXELPAL", 2, 3, OR, None);
    titre.contour(NUIT);
    t.coller(&titre, 5.0, 3.0, 1.0, false, 1.0);
    let mut v = Toile::new(60, 12);
    ecrire(&mut v, &format!("V{version}"), 2, 2, LAVANDE, None);
    v.contour(NUIT);
    t.coller(&v, 6.0, 15.0, 1.0, false, 1.0);
    // vignette
    for y in 0..SH {
        for x in 0..SW {
            let (dx, dy) = ((x as f32 / SW as f32 - 0.5) * 2.0, (y as f32 / SH as f32 - 0.5) * 2.0);
            let d = (dx * dx * 0.6 + dy * dy * 0.4).sqrt();
            if d > 0.75 { p(t, alpha(0xFF000000, (d - 0.75) * 0.9), x, y); }
        }
    }
}
