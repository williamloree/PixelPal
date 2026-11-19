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
pub const SW: i32 = 200;
pub const SH: i32 = 80;
const SOL: i32 = 63;
const VITESSE: f32 = 38.0;
const PERSO_X: f32 = 46.0;

fn obstacle(k: i64) -> f32 { k as f32 * 95.0 + 60.0 + h(k * 13) * 35.0 }

/// Hauteur de saut du personnage (0 au sol) : il saute par-dessus les rochers.
fn saut(temps: f32) -> f32 {
    let monde = temps * VITESSE + PERSO_X;
    let k = ((monde - 60.0) / 95.0).floor() as i64;
    for kk in [k, k + 1] {
        let d = obstacle(kk) - monde;
        if (-12.0..16.0).contains(&d) { return ((16.0 - d) / 28.0 * PI).sin() * 17.0; }
    }
    0.0
}

fn ciel(t: &mut Toile, temps: f32) {
    let degrade = [(0.0, 0xFF120A2A), (0.45, 0xFF2A1A5E), (0.75, 0xFF4B2F7E), (1.0, 0xFF7A4A8E)];
    for y in 0..SH {
        let k = y as f32 / SH as f32;
        let i = degrade.iter().rposition(|d| d.0 <= k).unwrap_or(0).min(degrade.len() - 2);
        let (a, b) = (degrade[i], degrade[i + 1]);
        t.rect(mel(a.1, b.1, (k - a.0) / (b.0 - a.0)), 0, y, SW, 1);
    }
    // aurore féerique
    for x in 0..SW {
        let y0 = 16.0 + (x as f32 * 0.045 + temps * 0.4).sin() * 4.0 + (x as f32 * 0.11 - temps * 0.7).sin() * 1.5;
        for j in 0..8 {
            let a = 0.10 * (1.0 - j as f32 / 8.0) * (0.6 + 0.4 * (temps * 0.8 + x as f32 * 0.03).sin());
            p(t, alpha(if (x / 40) % 2 == 0 { 0xFF6FE7FF } else { 0xFFD16BFF }, a), x, y0 as i32 + j);
        }
    }
    for i in 0..55 {
        let (x, y) = ((h(i) * SW as f32) as i32, (h(i + 100) * SH as f32 * 0.55) as i32);
        let sc = (((temps * (0.8 + h(i + 200) * 2.5)) + i as f32).sin() + 1.0) / 2.0;
        if sc < 0.25 { continue; }
        let c = if i % 4 == 0 { 0xFFFFE9A8 } else { 0xFFE9DCFF };
        p(t, alpha(c, sc), x, y);
        if i % 9 == 0 && sc > 0.8 { for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] { p(t, alpha(c, sc * 0.5), x + dx, y + dy); } }
    }
    // lune
    halo(t, 168.0, 15.0, 16.0, 0xFFE9DCFF, 0.35);
    disque(t, 168, 15, 7.5, 0xFFF6EBC8);
    for (x, y) in [(165, 13), (170, 17), (167, 18), (171, 12)] { p(t, 0xFFE2D3A8, x, y); }
    disque(t, 165, 13, 1.0, 0xFFE2D3A8);
}

fn montagnes(t: &mut Toile, temps: f32) {
    for x in 0..SW {
        let wx = x as f32 + temps * 3.0;
        let hy = 47.0 + (wx * 0.03).sin() * 5.0 + (wx * 0.071 + 1.0).sin() * 3.0;
        t.rect(0xFF261C4A, x, hy as i32, 1, SH);
        let hy2 = 53.0 + (wx * 1.4 * 0.04 + 2.0).sin() * 3.0;
        t.rect(0xFF1E1840, x, hy2 as i32, 1, SH);
    }
}

fn temple(t: &mut Toile, temps: f32) {
    let (cx, base) = (122, 58);
    let pierre = 0xFF3E3A5C;
    let (ombre, lumiere) = (0xFF2C2846, 0xFF5A5480);
    let etages = [64, 52, 41, 31, 22];
    let mut y = base;
    halo(t, cx as f32, (base - 34) as f32, 30.0, 0xFFF2C14E, 0.12 + 0.04 * (temps * 1.3).sin());
    for (i, &w) in etages.iter().enumerate() {
        let x0 = cx - w / 2;
        y -= 6;
        t.rect(pierre, x0, y, w, 6);
        t.rect(lumiere, x0, y, w, 1);
        t.rect(ombre, x0 + w - 3, y, 3, 6);
        t.rect(ombre, x0, y + 5, w, 1);
        // runes lumineuses
        let lueur = 0.5 + 0.5 * (temps * 2.0 + i as f32).sin();
        for r in 0..(w / 10) {
            let rx = x0 + 4 + r * 10;
            if (rx - cx).abs() < 6 { continue; }
            p(t, alpha(0xFF6FE7FF, 0.4 + 0.6 * lueur), rx, y + 2);
            p(t, alpha(0xFF6FE7FF, 0.3 + 0.5 * lueur), rx + 1, y + 3);
        }
        // mousse
        for m in 0..w / 6 {
            let mx = x0 + (h((i as i32 * 31 + m) as i64) * w as f32) as i32;
            p(t, if m % 2 == 0 { MOUSSE } else { 0xFF4F8F3A }, mx, y + (h((i as i32 * 7 + m) as i64) * 2.0) as i32);
        }
    }
    // escalier central
    for s in 0..30 { t.rect(if s % 2 == 0 { 0xFF4A4670 } else { lumiere }, cx - 4, base - 30 + s, 8, 1); }
    // sanctuaire et porte dorée
    let sy = y - 9;
    t.rect(pierre, cx - 8, sy, 16, 9);
    t.rect(lumiere, cx - 9, sy - 1, 18, 2);
    t.rect(ombre, cx + 6, sy, 2, 9);
    let pulse = 0.7 + 0.3 * (temps * 2.5).sin();
    t.rect(mel(0xFF8A5A14, OR, pulse), cx - 2, sy + 3, 5, 6);
    t.rect(mel(OR, OR_CLAIR, pulse), cx - 1, sy + 4, 3, 5);
    halo(t, cx as f32, (sy + 5) as f32, 9.0, OR, 0.35 * pulse);
    // torches
    for tx in [cx - 11, cx + 11] {
        t.rect(0xFF5C3A1F, tx, sy + 3, 1, 4);
        let f = ((temps * 12.0 + tx as f32).sin() * 1.2) as i32;
        p(t, 0xFFFFB347, tx, sy + 2);
        p(t, 0xFFFFE9A8, tx + f.signum(), sy + 1);
        p(t, alpha(0xFFFF7A2F, 0.8), tx, sy);
        halo(t, tx as f32, (sy + 1) as f32, 5.0, 0xFFFFB347, 0.35);
    }
}

fn plan_moyen(t: &mut Toile, temps: f32) {
    let defil = temps * 12.0;
    let premier = (defil / 46.0).floor() as i64 - 1;
    for seg in premier..premier + 7 {
        let x = (seg as f32 * 46.0 + h(seg) * 18.0 - defil) as i32;
        match (h(seg + 7) * 3.0) as i32 {
            0 => {
                // arbre sombre aux bords bleutés
                t.rect(0xFF1A2230, x + 5, 38, 3, 26);
                disque(t, x + 6, 34, 9.0, 0xFF1F3B2E);
                disque(t, x + 3, 37, 6.0, 0xFF1F3B2E);
                for a in 0..12 { let an = a as f32 / 12.0 * PI; p(t, 0xFF2D5A40, x + 6 + (an.cos() * 9.0) as i32, 34 - (an.sin() * 9.0) as i32); }
            }
            1 => {
                // colonne en ruine
                let hc = 14 + (h(seg + 3) * 10.0) as i32;
                t.rect(0xFF353150, x + 2, SOL - hc, 7, hc);
                t.rect(0xFF46426A, x + 2, SOL - hc, 2, hc);
                t.rect(0xFF46426A, x, SOL - hc - 2, 11, 2);
                for m in 0..4 { p(t, 0xFF4F8F3A, x + 2 + (h(seg * 5 + m) * 7.0) as i32, SOL - hc + (h(seg * 3 + m) * hc as f32) as i32); }
            }
            _ => {
                // champignon géant lumineux
                let c = if seg % 2 == 0 { 0xFFB04FD8 } else { 0xFF3FB4CC };
                let lueur = 0.6 + 0.4 * (temps * 1.7 + seg as f32).sin();
                halo(t, (x + 6) as f32, 44.0, 12.0, c, 0.25 * lueur);
                t.rect(0xFFD8CFE8, x + 5, 46, 3, SOL - 46);
                t.rect(0xFFB8AECD, x + 7, 46, 1, SOL - 46);
                for r in 0..5 { let w = 13 - r * 2; t.rect(c, x + 6 - w / 2 - 1 + 1, 46 - r, w, 1); }
                t.rect(mel(c, 0xFF000000, 0.3), x, 46, 13, 1);
                for (dx, dy) in [(2, -2), (6, -4), (9, -2)] { p(t, alpha(0xFFFFFFFF, 0.6 + 0.4 * lueur), x + dx, 46 + dy); }
            }
        }
    }
}

fn lianes(t: &mut Toile, temps: f32) {
    for i in 0..8 {
        let x0 = h(i + 300) * SW as f32;
        let long = 8 + (h(i + 400) * 24.0) as i32;
        for y in 0..long {
            let x = x0 + (temps * 1.4 + y as f32 * 0.18 + i as f32).sin() * 1.3 * (y as f32 / long as f32);
            p(t, 0xFF2D5A2E, x as i32, y);
            if y % 3 == 1 { p(t, if y % 2 == 0 { 0xFF4F8F3A } else { FEUILLE }, x as i32 + if y % 6 == 1 { 1 } else { -1 }, y); }
        }
        if i % 2 == 0 {
            let x = x0 + (temps * 1.4 + long as f32 * 0.18 + i as f32).sin() * 1.3;
            let l = 0.5 + 0.5 * (temps * 2.0 + i as f32).sin();
            halo(t, x, long as f32 + 1.0, 4.0, 0xFFFF7AD9, 0.5 * l);
            p(t, 0xFFFF9AE3, x as i32, long);
        }
    }
}

fn sol(t: &mut Toile, temps: f32) {
    let defil = temps * VITESSE;
    // chemin de briques
    for rang in 0..4 {
        let y = SOL + 1 + rang * 5;
        let dec = if rang % 2 == 0 { 0.0 } else { 6.0 };
        let premier = ((defil - dec) / 12.0).floor() as i64 - 1;
        for b in premier..premier + 19 {
            let x = (b as f32 * 12.0 + dec - defil).round() as i32;
            let c = mel(0xFF4A4466, 0xFF3C3656, h(b * 3 + rang as i64));
            t.rect(0xFF2A2540, x, y, 12, 5);
            t.rect(c, x + 1, y, 11, 4);
            t.rect(mel(c, 0xFF6A6490, 0.4), x + 1, y, 11, 1);
        }
    }
    // herbe et fleurs lumineuses
    t.rect(MOUSSE, 0, SOL, SW, 1);
    let premier = (defil / 4.0).floor() as i64;
    for k in premier..premier + 52 {
        let x = (k as f32 * 4.0 - defil) as i32;
        let hh = (h(k) * 3.0) as i32;
        for j in 0..=hh { p(t, if j == hh { 0xFF5C9A45 } else { MOUSSE }, x + (j % 2), SOL - j); }
        if h(k + 900) > 0.86 {
            let c = if k % 3 == 0 { 0xFFFF7AD9 } else if k % 3 == 1 { 0xFF6FE7FF } else { 0xFFFFE066 };
            let l = 0.6 + 0.4 * (temps * 3.0 + k as f32).sin();
            halo(t, x as f32, (SOL - 4) as f32, 3.5, c, 0.5 * l);
            p(t, 0xFF3F6B3A, x, SOL - 1); p(t, 0xFF3F6B3A, x, SOL - 2);
            p(t, c, x, SOL - 3); p(t, mel(c, 0xFFFFFFFF, 0.5), x, SOL - 4);
            p(t, c, x - 1, SOL - 3); p(t, c, x + 1, SOL - 3);
        }
    }
    // rochers moussus à sauter
    let monde = defil;
    let k0 = ((monde - 60.0) / 95.0).floor() as i64 - 1;
    for k in k0..k0 + 4 {
        let x = (obstacle(k) - monde) as i32;
        t.rect(NUIT, x - 5, SOL - 6, 11, 7);
        t.rect(0xFF57527A, x - 4, SOL - 5, 9, 6);
        t.rect(0xFF6E6896, x - 3, SOL - 5, 5, 2);
        t.rect(FEUILLE, x - 4, SOL - 6, 4, 1);
        p(t, 0xFF4F8F3A, x - 5, SOL - 5);
    }
}

fn fee(t: &mut Toile, temps: f32, saut: f32) {
    let pos = |tm: f32| (PERSO_X + 24.0 + (tm * 1.3).sin() * 10.0, 30.0 + (tm * 2.1).sin() * 6.0 - saut * 0.3);
    for k in 1..10 {
        let (x, y) = pos(temps - k as f32 * 0.05);
        let c = if k % 2 == 0 { 0xFFFFE9A8 } else { 0xFFFF9AE3 };
        p(t, alpha(c, 0.7 - k as f32 * 0.07), (x - k as f32 * 2.5) as i32, (y + (k as f32 * 1.7 + temps * 9.0).sin() * 1.5) as i32);
    }
    let (x, y) = pos(temps);
    halo(t, x, y, 7.0, 0xFFFF9AE3, 0.55 + 0.15 * (temps * 6.0).sin());
    let aile = ((temps * 22.0) as i32) % 2;
    let ca = alpha(0xFFBFF6FF, 0.85);
    p(t, ca, x as i32 - 1, y as i32 - 1 - aile); p(t, ca, x as i32 - 2, y as i32 - 2 + aile);
    p(t, ca, x as i32 + 2, y as i32 - 1 - aile); p(t, ca, x as i32 + 3, y as i32 - 2 + aile);
    t.rect(0xFFFFFFFF, x as i32, y as i32, 2, 2);
}

fn lucioles(t: &mut Toile, temps: f32) {
    for i in 0..14 {
        let x = (h(i + 600) * SW as f32 + (temps * 0.3 + i as f32).sin() * 14.0 - temps * 5.0 * (0.4 + h(i + 5))).rem_euclid(SW as f32);
        let y = 30.0 + h(i + 700) * 30.0 + (temps * 0.8 + i as f32 * 2.0).sin() * 4.0;
        let l = (temps * 2.0 + i as f32 * 1.7).sin().max(0.0);
        if l < 0.05 { continue; }
        halo(t, x, y, 3.5, 0xFFD8FF7A, 0.6 * l);
        p(t, alpha(0xFFF4FFC8, l), x as i32, y as i32);
    }
}

/// Dessine une image de la scène (SW × SH pixels).
pub fn scene(t: &mut Toile, temps: f32, sprites: &mut Sprites, look: &str, version: &str) {
    ciel(t, temps);
    montagnes(t, temps);
    temple(t, temps);
    // brume
    for x in 0..SW {
        let y = 55.0 + (x as f32 * 0.07 + temps * 0.6).sin() * 1.5;
        for j in 0..7 { p(t, alpha(0xFFB9A8E0, 0.10 * (1.0 - (j as f32 - 3.0).abs() / 4.0)), x, y as i32 + j); }
    }
    plan_moyen(t, temps);
    sol(t, temps);
    let s = saut(temps);
    // ombre du personnage
    let largeur_ombre = (10.0 - s * 0.3).max(4.0) as i32;
    t.rect(alpha(0xFF000000, 0.35), PERSO_X as i32 - largeur_ombre / 2, SOL, largeur_ombre, 1);
    let img = if s > 0.5 { sprites.image_n(look, "jump", 2) } else { sprites.image(look, "walk", temps * 1.5) };
    let img = img.clone();
    t.coller(&img, PERSO_X - crate::sprites::CX as f32, SOL as f32 - crate::sprites::BAS as f32 - s, 1.0, false, 1.0);
    fee(t, temps, s);
    lucioles(t, temps);
    lianes(t, temps);
    // titre doré qui ondule
    let mut titre = Toile::new(60, 14);
    ecrire(&mut titre, "PIXELPAL", 2, 3, OR, Some(temps));
    titre.contour(NUIT);
    for c in titre.px.iter_mut() { if *c == OR && (temps * 1.5).sin() > 0.95 { *c = OR_CLAIR; } }
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
