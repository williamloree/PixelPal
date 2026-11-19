//! Toile de pixels ARGB (non prémultipliés) : rectangles, collage mis à l'échelle, détourage.

#[derive(Clone)]
pub struct Toile {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>, // 0xAARRGGBB
}

impl Toile {
    pub fn new(w: i32, h: i32) -> Self {
        let (w, h) = (w.max(1), h.max(1));
        Toile { w, h, px: vec![0; (w * h) as usize] }
    }

    /// Rectangle plein ; la couleur peut être semi-transparente (mélange par-dessus).
    pub fn rect(&mut self, c: u32, x: i32, y: i32, w: i32, h: i32) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.w), (y + h).min(self.h));
        let opaque = c >> 24 == 255;
        for yy in y0..y1 {
            let l = (yy * self.w) as usize;
            for xx in x0..x1 {
                let i = l + xx as usize;
                self.px[i] = if opaque { c } else { melanger(self.px[i], c) };
            }
        }
    }

    /// Colle `src` en (dx, dy), agrandie d'un facteur `k` (plus proche voisin), éventuellement en miroir.
    pub fn coller(&mut self, src: &Toile, dx: f32, dy: f32, k: f32, miroir: bool, alpha: f32) {
        if k <= 0.01 || alpha <= 0.01 { return; }
        let (lw, lh) = (src.w as f32 * k, src.h as f32 * k);
        let x0 = dx.floor().max(0.0) as i32;
        let y0 = dy.floor().max(0.0) as i32;
        let x1 = ((dx + lw).ceil() as i32).min(self.w);
        let y1 = ((dy + lh).ceil() as i32).min(self.h);
        for y in y0..y1 {
            let sy = ((y as f32 + 0.5 - dy) / k).floor() as i32;
            if sy < 0 || sy >= src.h { continue; }
            for x in x0..x1 {
                let mut sx = ((x as f32 + 0.5 - dx) / k).floor() as i32;
                if sx < 0 || sx >= src.w { continue; }
                if miroir { sx = src.w - 1 - sx; }
                let mut c = src.px[(sy * src.w + sx) as usize];
                if c >> 24 == 0 { continue; }
                if alpha < 1.0 { c = (c & 0x00FF_FFFF) | ((((c >> 24) as f32 * alpha) as u32) << 24); }
                let i = (y * self.w + x) as usize;
                self.px[i] = if c >> 24 == 255 { c } else { melanger(self.px[i], c) };
            }
        }
    }

    /// Rotation d'un quart de tour vers la gauche (le personnage se retrouve couché sur le dos).
    pub fn pivoter_gauche(&self) -> Toile {
        let mut t = Toile::new(self.h, self.w);
        for y in 0..self.h { for x in 0..self.w { t.px[((self.w - 1 - x) * t.w + y) as usize] = self.px[(y * self.w + x) as usize]; } }
        t
    }

    /// Retournement haut-bas (marche au plafond).
    pub fn retourner_v(&self) -> Toile {
        let mut t = Toile::new(self.w, self.h);
        for y in 0..self.h { t.px[(y * self.w) as usize..((y + 1) * self.w) as usize].copy_from_slice(&self.px[((self.h - 1 - y) * self.w) as usize..((self.h - y) * self.w) as usize]); }
        t
    }

    /// Version fantôme : pâle, bleutée, translucide.
    pub fn fantome(&self, alpha: f32) -> Toile {
        let mut t = self.clone();
        for c in &mut t.px {
            if *c >> 24 == 0 { continue; }
            let l = (((*c >> 16) & 255) + ((*c >> 8) & 255) + (*c & 255)) / 3;
            let (r, g, b) = ((l + 400) / 3, (l + 440) / 3, (l * 2 + 255 * 2) / 4);
            *c = (((alpha * 255.0) as u32) << 24) | (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
        }
        t
    }

    /// Ajoute un contour d'un pixel autour de tout ce qui est opaque.
    pub fn contour(&mut self, c: u32) {
        let src = self.px.clone();
        let plein = |x: i32, y: i32| x >= 0 && y >= 0 && x < self.w && y < self.h && src[(y * self.w + x) as usize] >> 24 != 0;
        let mut ajout = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if src[(y * self.w + x) as usize] >> 24 != 0 { continue; }
                if plein(x - 1, y) || plein(x + 1, y) || plein(x, y - 1) || plein(x, y + 1) { ajout.push((y * self.w + x) as usize); }
            }
        }
        for i in ajout { self.px[i] = c; }
    }

    /// Pixels BGRA prémultipliés, pour UpdateLayeredWindow.
    pub fn vers_bgra_premul(&self, dst: &mut [u8]) {
        for (i, &c) in self.px.iter().enumerate() {
            let a = c >> 24;
            let m = |v: u32| ((v * a + 127) / 255) as u8;
            dst[i * 4] = m(c & 255);
            dst[i * 4 + 1] = m((c >> 8) & 255);
            dst[i * 4 + 2] = m((c >> 16) & 255);
            dst[i * 4 + 3] = a as u8;
        }
    }

    /// Octets RGBA non prémultipliés (egui, PNG).
    pub fn vers_rgba(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.px.len() * 4);
        for &c in &self.px {
            v.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8]);
        }
        v
    }
}

fn melanger(fond: u32, c: u32) -> u32 {
    let a = (c >> 24) as f32 / 255.0;
    let fa = (fond >> 24) as f32 / 255.0;
    let oa = a + fa * (1.0 - a);
    if oa <= 0.0 { return 0; }
    let canal = |s: u32| {
        let (cs, cf) = (((c >> s) & 255) as f32, ((fond >> s) & 255) as f32);
        (((cs * a + cf * fa * (1.0 - a)) / oa).round() as u32).min(255) << s
    };
    ((oa * 255.0).round() as u32) << 24 | canal(16) | canal(8) | canal(0)
}
