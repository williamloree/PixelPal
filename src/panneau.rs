//! Panneau (fenêtre egui, processus séparé) : messages, groupes, contacts, écrans, personnage et storyboard.
//! Thème « temple au jardin féerique » : bannière animée, dalles de pierre, titres en police pixel.
//! Il pilote le programme principal par des commandes « ctl » en local.

use crate::reseau::requete;
use crate::scene::{self, CREME, LAVANDE, NUIT, OR, OR_CLAIR};
use crate::sprites::{self, Sprites};
use crate::toile::Toile;
use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, pos2, vec2};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn c(argb: u32) -> Color32 { Color32::from_rgb((argb >> 16) as u8, (argb >> 8) as u8, argb as u8) }
fn ca(argb: u32, a: f32) -> Color32 { Color32::from_rgba_unmultiplied((argb >> 16) as u8, (argb >> 8) as u8, argb as u8, (a.clamp(0.0, 1.0) * 255.0) as u8) }

const PIERRE: u32 = 0xFF2E2850;
const PIERRE_CLAIRE: u32 = 0xFF4B4470;
const PIERRE_SOMBRE: u32 = 0xFF17132A;
const VIOLET: u32 = 0xFF4B2F7E;
const CARTE: u32 = 0xFF1F1B33;

pub fn lancer(port: u16, jeton: String) -> eframe::Result {
    let img = Sprites::default().image_n("aventurier", "idle", 0).clone();
    let icone = egui::IconData { rgba: img.vers_rgba(), width: img.w as u32, height: img.h as u32 };
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 760.0]).with_min_inner_size([560.0, 600.0])
            .with_title("PixelPal").with_icon(icone),
        ..Default::default()
    };
    eframe::run_native("PixelPal", opts, Box::new(move |cc| {
        theme(&cc.egui_ctx);
        Ok(Box::new(Panneau::new(port, jeton)))
    }))
}

fn theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        let mut v = egui::Visuals::dark();
        v.panel_fill = c(0xFF151226);
        v.window_fill = c(CARTE);
        v.extreme_bg_color = c(0xFF0E0C1A);
        v.faint_bg_color = c(0xFF1F1B33);
        v.override_text_color = Some(c(CREME));
        v.hyperlink_color = c(OR);
        v.selection.bg_fill = ca(0xFFD16BFF, 0.35);
        v.selection.stroke = Stroke::new(1.0, c(OR));
        v.window_corner_radius = CornerRadius::ZERO;
        v.menu_corner_radius = CornerRadius::ZERO;
        v.window_stroke = Stroke::new(2.0, c(PIERRE_CLAIRE));
        let w = &mut v.widgets;
        for x in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] { x.corner_radius = CornerRadius::ZERO; }
        w.noninteractive.bg_fill = c(CARTE);
        w.noninteractive.bg_stroke = Stroke::new(1.0, c(0xFF3B3560));
        w.noninteractive.fg_stroke = Stroke::new(1.0, c(CREME));
        w.inactive.bg_fill = c(PIERRE);
        w.inactive.weak_bg_fill = c(PIERRE);
        w.inactive.bg_stroke = Stroke::new(1.0, c(PIERRE_CLAIRE));
        w.inactive.fg_stroke = Stroke::new(1.0, c(CREME));
        w.hovered.bg_fill = c(0xFF3B3366);
        w.hovered.weak_bg_fill = c(0xFF3B3366);
        w.hovered.bg_stroke = Stroke::new(1.5, c(OR));
        w.hovered.fg_stroke = Stroke::new(1.5, c(OR_CLAIR));
        w.active.bg_fill = c(VIOLET);
        w.active.weak_bg_fill = c(VIOLET);
        w.active.bg_stroke = Stroke::new(1.5, c(OR_CLAIR));
        w.open = w.active;
        s.spacing.item_spacing = vec2(8.0, 7.0);
        s.spacing.button_padding = vec2(10.0, 5.0);
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Body, FontId::proportional(15.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(15.0));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(20.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.5));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(14.0));
    });
}

#[derive(PartialEq, Clone, Copy)]
enum Onglet { Message, Groupes, Contacts, Ecrans, Personnage }

struct Panneau {
    port: u16,
    jeton: String,
    etat: Value,
    partage: Arc<Mutex<Option<Value>>>,
    rafraichir: Arc<AtomicBool>,
    onglet: Onglet,
    retour: String,
    retour_t: Instant,
    groupe: String,
    tous: bool,
    choix: HashSet<String>,
    texte: String,
    nom_groupe: String,
    choix_groupe: HashSet<String>,
    code: String,
    nom: String,
    look: String,
    profil_charge: bool,
    sprites: Sprites,
    textures: HashMap<(String, &'static str, usize), TextureHandle>,
    etiquettes: HashMap<(String, u32), TextureHandle>,
    toile_scene: Toile,
    tex_scene: Option<TextureHandle>,
    tex_dalle: Option<TextureHandle>,
    debut: Instant,
}

impl Panneau {
    fn new(port: u16, jeton: String) -> Self {
        // l'état est lu dans un fil à part : l'interface ne gèle jamais, même si PixelPal tarde à répondre
        let partage = Arc::new(Mutex::new(None));
        let rafraichir = Arc::new(AtomicBool::new(true));
        let (p2, r2, j2) = (partage.clone(), rafraichir.clone(), jeton.clone());
        std::thread::spawn(move || loop {
            r2.store(false, Ordering::Relaxed);
            let v = json!({"type": "ctl", "jeton": j2, "cmd": "etat"});
            let e = requete("127.0.0.1", port, &v, Duration::from_secs(2))
                .unwrap_or_else(|e| json!({"ok": false, "erreur": format!("PixelPal ne répond pas ({e})")}));
            *p2.lock().unwrap() = Some(e);
            for _ in 0..15 {
                if r2.load(Ordering::Relaxed) { break; }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        Panneau { port, jeton, etat: Value::Null, partage, rafraichir, onglet: Onglet::Message, retour: String::new(),
            retour_t: Instant::now(), groupe: String::new(), tous: true, choix: HashSet::new(), texte: String::new(), nom_groupe: String::new(),
            choix_groupe: HashSet::new(), code: String::new(), nom: String::new(), look: "aventurier".into(), profil_charge: false,
            sprites: Sprites::default(), textures: HashMap::new(), etiquettes: HashMap::new(), toile_scene: Toile::new(scene::SW, scene::SH),
            tex_scene: None, tex_dalle: None, debut: Instant::now() }
    }

    fn temps(&self) -> f32 { self.debut.elapsed().as_secs_f32() }

    fn appeler(&self, cmd: &str, mut extra: Value) -> Value {
        if !extra.is_object() { extra = json!({}); }
        extra["type"] = json!("ctl");
        extra["jeton"] = json!(self.jeton);
        extra["cmd"] = json!(cmd);
        requete("127.0.0.1", self.port, &extra, Duration::from_secs(2)).unwrap_or_else(|e| json!({"ok": false, "erreur": format!("PixelPal ne répond pas ({e})")}))
    }

    fn dire(&mut self, s: String) { self.retour = s; self.retour_t = Instant::now(); }

    /// Lance une commande et garde le message de retour.
    fn action(&mut self, cmd: &str, extra: Value, succes: &str) -> bool {
        let r = self.appeler(cmd, extra);
        let ok = r["ok"] == true;
        self.dire(if ok { succes.to_string() } else { format!("⚠ {}", r["erreur"].as_str().unwrap_or("échec")) });
        self.rafraichir.store(true, Ordering::Relaxed);
        ok
    }

    // ---------- textures ----------
    fn texture(&mut self, ctx: &egui::Context, look: &str, anim: &'static str, i: usize) -> TextureHandle {
        let cle = (look.to_string(), anim, i);
        if let Some(t) = self.textures.get(&cle) { return t.clone(); }
        let img = self.sprites.image_n(look, anim, i);
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.w as usize, img.h as usize], &img.vers_rgba());
        let t = ctx.load_texture(format!("{look}-{anim}-{i}"), ci, TextureOptions::NEAREST);
        self.textures.insert(cle, t.clone());
        t
    }

    fn charger(ctx: &egui::Context, nom: &str, t: &Toile, opt: TextureOptions) -> TextureHandle {
        ctx.load_texture(nom, egui::ColorImage::from_rgba_unmultiplied([t.w as usize, t.h as usize], &t.vers_rgba()), opt)
    }

    /// Texte en police pixel (mis en cache).
    fn etiquette(&mut self, ctx: &egui::Context, s: &str, coul: u32) -> TextureHandle {
        let cle = (s.to_string(), coul);
        if let Some(t) = self.etiquettes.get(&cle) { return t.clone(); }
        let t = Self::charger(ctx, &format!("txt-{s}-{coul}"), &scene::etiquette(s, coul, NUIT), TextureOptions::NEAREST);
        self.etiquettes.insert(cle, t.clone());
        t
    }

    fn icone(&mut self, ctx: &egui::Context, nom: &str, coul: u32) -> TextureHandle {
        let cle = (format!("@{nom}"), coul);
        if let Some(t) = self.etiquettes.get(&cle) { return t.clone(); }
        let t = Self::charger(ctx, &format!("ico-{nom}-{coul}"), &scene::icone(nom, coul), TextureOptions::NEAREST);
        self.etiquettes.insert(cle, t.clone());
        t
    }

    fn titre(&mut self, ui: &mut Ui, s: &str) {
        let t = self.etiquette(ui.ctx(), s, OR);
        let k = 2.0;
        ui.add(egui::Image::new(&t).fit_to_exact_size(vec2(t.size()[0] as f32 * k, t.size()[1] as f32 * k)));
    }

    fn image_animee(&mut self, ui: &mut Ui, look: &str, anim: &'static str, taille: f32) -> Rect {
        let i = sprites::index_image(anim, self.temps() % 6.0);
        let tex = self.texture(ui.ctx(), look, anim, i);
        ui.add(egui::Image::new(&tex).fit_to_exact_size(vec2(taille, taille))).rect
    }

    /// Petite estrade (ciel nocturne + herbe) pour présenter un sprite.
    fn estrade(&mut self, ui: &mut Ui, look: &str, anim: &'static str, taille: f32) {
        let (r, _) = ui.allocate_exact_size(vec2(taille, taille), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 0.0, c(0xFF231A4A));
        p.rect_filled(Rect::from_min_max(pos2(r.min.x, r.min.y + taille * 0.55), r.max), 0.0, c(0xFF2A1F55));
        let sol = r.min.y + taille * (sprites::BAS as f32 + 1.0) / sprites::H as f32;
        p.rect_filled(Rect::from_min_max(pos2(r.min.x, sol - 1.0), pos2(r.max.x, sol + 3.0)), 0.0, c(scene::MOUSSE));
        p.rect_filled(Rect::from_min_max(pos2(r.min.x, sol + 3.0), r.max), 0.0, c(0xFF3C3656));
        let i = sprites::index_image(anim, self.temps() % 6.0);
        let tex = self.texture(ui.ctx(), look, anim, i);
        ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(2.0, c(PIERRE_CLAIRE)), StrokeKind::Inside);
    }

    // ---------- éléments de décor ----------
    fn fond(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let tex = self.tex_dalle.get_or_insert_with(|| Self::charger(ui.ctx(), "dalle", &scene::dalle(), TextureOptions::NEAREST_REPEAT)).clone();
        let k = 96.0;
        ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(r.width() / k, r.height() / k)), Color32::from_gray(170));
        // lucioles qui dérivent
        let t = self.temps();
        for i in 0..16 {
            let f = i as f32;
            let x = r.min.x + ((f * 97.3 + (t * 0.3 + f).sin() * 30.0 + t * (6.0 + f % 5.0)) % r.width()).abs();
            let y = r.min.y + 260.0 + ((f * 61.7 + (t * 0.5 + f * 2.0).sin() * 20.0) % (r.height() - 260.0).max(1.0)).abs();
            let l = (t * 1.7 + f * 1.3).sin().max(0.0);
            if l < 0.05 { continue; }
            ui.painter().circle_filled(pos2(x, y), 7.0, ca(0xFFD8FF7A, 0.10 * l));
            ui.painter().rect_filled(Rect::from_center_size(pos2(x, y), vec2(3.0, 3.0)), 0.0, ca(0xFFF4FFC8, 0.9 * l));
        }
    }

    /// Feuillages et fleurs luisantes aux coins d'un cadre.
    fn ornements(&self, p: &egui::Painter, r: Rect) {
        let t = self.temps();
        let px = 3.0;
        let carre = |x: f32, y: f32, col: Color32| { p.rect_filled(Rect::from_min_size(pos2(x, y), vec2(px, px)), 0.0, col); };
        for (i, (cx, cy, sx, sy)) in [(r.min.x, r.min.y, 1.0, 1.0), (r.max.x, r.min.y, -1.0, 1.0), (r.min.x, r.max.y, 1.0, -1.0), (r.max.x, r.max.y, -1.0, -1.0)].into_iter().enumerate() {
            let o = |dx: f32, dy: f32| (cx + sx * dx * px - if sx < 0.0 { px } else { 0.0 }, cy + sy * dy * px - if sy < 0.0 { px } else { 0.0 });
            for (dx, dy, col) in [(0.0, 0.0, 0xFF3F6B3A), (1.0, 0.0, 0xFF6FBF4F), (2.0, 0.0, 0xFF3F6B3A), (3.0, 1.0, 0xFF4F8F3A), (0.0, 1.0, 0xFF6FBF4F), (0.0, 2.0, 0xFF3F6B3A), (1.0, 3.0, 0xFF4F8F3A), (1.0, 1.0, 0xFF2D5A2E)] {
                let (x, y) = o(dx, dy);
                carre(x, y, c(col));
            }
            let l = 0.5 + 0.5 * (t * 2.0 + i as f32 * 1.6).sin();
            let fleur = if i % 2 == 0 { 0xFFFF7AD9 } else { 0xFF6FE7FF };
            let (x, y) = o(2.0, 2.0);
            p.circle_filled(pos2(x + px / 2.0, y + px / 2.0), 6.0, ca(fleur, 0.25 * l));
            carre(x, y, ca(fleur, 0.6 + 0.4 * l));
        }
    }

    fn cadre() -> egui::Frame {
        egui::Frame::new().fill(ca(CARTE, 0.94)).stroke(Stroke::new(2.0, c(PIERRE_CLAIRE))).inner_margin(14.0).corner_radius(0.0)
    }

    /// Gros bouton doré en pixel art.
    fn bouton_or(&mut self, ui: &mut Ui, texte: &str) -> bool {
        let lab = self.etiquette(ui.ctx(), texte, NUIT);
        let k = 2.0;
        let taille = vec2(lab.size()[0] as f32 * k + 36.0, 40.0);
        let (r, rep) = ui.allocate_exact_size(taille, Sense::click());
        let survol = rep.hovered();
        let enfonce = rep.is_pointer_button_down_on();
        let r = r.translate(vec2(0.0, if enfonce { 2.0 } else if survol { -1.0 } else { 0.0 }));
        let p = ui.painter();
        if survol { p.rect_filled(r.expand(5.0), 0.0, ca(OR, 0.15)); }
        p.rect_filled(r, 0.0, c(NUIT));
        let inter = r.shrink(2.0);
        p.rect_filled(inter, 0.0, c(if survol { OR_CLAIR } else { OR }));
        p.rect_filled(Rect::from_min_size(inter.min, vec2(inter.width(), 3.0)), 0.0, c(0xFFFFF6D6));
        p.rect_filled(Rect::from_min_max(pos2(inter.min.x, inter.max.y - 4.0), inter.max), 0.0, c(0xFFB0782A));
        // étincelles quand on survole
        if survol {
            let t = self.temps();
            for i in 0..3 {
                let x = r.min.x + (t * 40.0 + i as f32 * 53.0) % r.width();
                p.rect_filled(Rect::from_center_size(pos2(x, r.min.y - 4.0 - (t * 5.0 + i as f32).sin().abs() * 4.0), vec2(3.0, 3.0)), 0.0, c(OR_CLAIR));
            }
        }
        let tl = vec2(lab.size()[0] as f32 * k, lab.size()[1] as f32 * k);
        p.image(lab.id(), Rect::from_center_size(r.center() + vec2(0.0, -1.0), tl), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        if survol { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        rep.clicked()
    }

    /// Onglet en forme de tablette de pierre, avec icône.
    fn tablette(&mut self, ui: &mut Ui, ico: &str, texte: &str, actif: bool) -> bool {
        let coul = if actif { OR } else { CREME };
        let lab = self.etiquette(ui.ctx(), texte, coul);
        let ic = self.icone(ui.ctx(), ico, coul);
        let k = 2.0;
        let taille = vec2(lab.size()[0] as f32 * k + ic.size()[0] as f32 * k + 22.0, 42.0);
        let (r, rep) = ui.allocate_exact_size(taille, Sense::click());
        let survol = rep.hovered();
        let r = if actif { r.translate(vec2(0.0, -2.0)) } else { r };
        let p = ui.painter();
        if actif {
            let l = 0.5 + 0.5 * (self.temps() * 2.0).sin();
            p.rect_filled(r.expand(4.0), 0.0, ca(0xFFD16BFF, 0.10 + 0.08 * l));
        }
        p.rect_filled(r, 0.0, c(NUIT));
        let inter = r.shrink(2.0);
        p.rect_filled(inter, 0.0, c(if actif { VIOLET } else if survol { 0xFF3B3366 } else { PIERRE }));
        p.rect_filled(Rect::from_min_size(inter.min, vec2(inter.width(), 2.0)), 0.0, c(if actif { 0xFF7A5BB5 } else { PIERRE_CLAIRE }));
        p.rect_filled(Rect::from_min_max(pos2(inter.min.x, inter.max.y - 3.0), inter.max), 0.0, c(PIERRE_SOMBRE));
        if actif || survol { p.rect_stroke(r, 0.0, Stroke::new(1.5, c(if actif { OR } else { PIERRE_CLAIRE })), StrokeKind::Outside); }
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let ti = vec2(ic.size()[0] as f32 * k, ic.size()[1] as f32 * k);
        let ri = Rect::from_min_size(pos2(r.min.x + 8.0, r.center().y - ti.y / 2.0), ti);
        p.image(ic.id(), ri, uv, Color32::WHITE);
        let tl = vec2(lab.size()[0] as f32 * k, lab.size()[1] as f32 * k);
        p.image(lab.id(), Rect::from_min_size(pos2(ri.max.x + 4.0, r.center().y - tl.y / 2.0), tl), uv, Color32::WHITE);
        if survol { ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand); }
        rep.clicked()
    }

    fn banniere(&mut self, ui: &mut Ui) {
        let version = self.etat["version"].as_str().unwrap_or(crate::maj::VERSION).to_string();
        let look = self.look.clone();
        let t = self.temps();
        scene::scene(&mut self.toile_scene, t, &mut self.sprites, &look, &version);
        let ci = egui::ColorImage::from_rgba_unmultiplied([scene::SW as usize, scene::SH as usize], &self.toile_scene.vers_rgba());
        match &mut self.tex_scene {
            Some(tex) => tex.set(ci, TextureOptions::NEAREST),
            None => self.tex_scene = Some(ui.ctx().load_texture("scene", ci, TextureOptions::NEAREST)),
        }
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, w * scene::SH as f32 / scene::SW as f32), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r.expand(6.0), 0.0, c(NUIT));
        p.rect_filled(r.expand(4.0), 0.0, c(PIERRE_CLAIRE));
        p.rect_filled(r.expand(2.0), 0.0, c(NUIT));
        p.image(self.tex_scene.as_ref().unwrap().id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        self.ornements(ui.painter(), r.expand(6.0));
        if let Some(p) = self.etat["profil_arg"].as_str() {
            let lab = self.etiquette(ui.ctx(), &format!("PROFIL DE TEST {p}"), LAVANDE);
            let s = vec2(lab.size()[0] as f32 * 2.0, lab.size()[1] as f32 * 2.0);
            ui.painter().image(lab.id(), Rect::from_min_size(pos2(r.max.x - s.x - 8.0, r.max.y - s.y - 6.0), s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
    }

    fn contacts(&self) -> Vec<Value> { self.etat["contacts"].as_array().cloned().unwrap_or_default() }
    fn en_ligne(&self, id: &str) -> Option<bool> { self.contacts().iter().find(|c| c["id"] == id).map(|c| c["en_ligne"] == true) }

    // ---------- onglets ----------
    fn onglet_message(&mut self, ui: &mut Ui) {
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "ENVOYER UNE LETTRE");
            let groupes = self.etat["groupes"].as_array().cloned().unwrap_or_default();
            if groupes.is_empty() {
                ui.label("Pour envoyer un message, crée d'abord un groupe (onglet Groupes) avec les personnes qui ont PixelPal.");
                return;
            }
            if !groupes.iter().any(|g| g["id"] == self.groupe.as_str()) { self.groupe = groupes[0]["id"].as_str().unwrap_or("").into(); }
            let g = groupes.iter().find(|g| g["id"] == self.groupe.as_str()).cloned().unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label("Groupe :");
                egui::ComboBox::from_id_salt("groupe").selected_text(g["nom"].as_str().unwrap_or("")).show_ui(ui, |ui| {
                    for x in &groupes { ui.selectable_value(&mut self.groupe, x["id"].as_str().unwrap_or("").to_string(), x["nom"].as_str().unwrap_or("")); }
                });
            });
            let moi = self.etat["profil"]["id"].as_str().unwrap_or("").to_string();
            let membres: Vec<Value> = g["membres"].as_array().cloned().unwrap_or_default().into_iter().filter(|m| m["id"] != moi.as_str()).collect();
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.tous, true, "Tout le groupe");
                ui.radio_value(&mut self.tous, false, "Seulement…");
            });
            if !self.tous {
                ui.horizontal_wrapped(|ui| {
                    for m in &membres {
                        let id = m["id"].as_str().unwrap_or("").to_string();
                        let mut coche = self.choix.contains(&id);
                        let pastille = if self.en_ligne(&id) == Some(true) { "🟢" } else { "⚪" };
                        if ui.toggle_value(&mut coche, format!("{pastille} {}", m["nom"].as_str().unwrap_or("?"))).changed() {
                            if coche { self.choix.insert(id); } else { self.choix.remove(&id); }
                        }
                    }
                });
            }
            ui.add_space(4.0);
            ui.add(egui::TextEdit::multiline(&mut self.texte).hint_text("Écris ta lettre…").desired_rows(5).desired_width(f32::INFINITY).margin(vec2(10.0, 8.0)));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if self.bouton_or(ui, "ENVOYER PAR PORTAIL") {
                    let a = if self.tous { Value::Null } else { json!(self.choix.iter().collect::<Vec<_>>()) };
                    let v = json!({"groupe": self.groupe, "a": a, "texte": self.texte});
                    if self.action("envoyer", v, "Ton personnage part livrer la lettre !") { self.texte.clear(); }
                }
            });
            ui.add_space(2.0);
            ui.small("Il ouvre un portail et y entre : la lettre arrive chez chaque destinataire en même temps, portée par un visiteur qui sort d'un portail sur leur écran.");
        });
        self.ornements(ui.painter(), rep.response.rect);
    }

    fn onglet_groupes(&mut self, ui: &mut Ui) {
        let groupes = self.etat["groupes"].as_array().cloned().unwrap_or_default();
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "MES GROUPES");
            if groupes.is_empty() { ui.label("Aucun groupe pour l'instant."); }
            for g in &groupes {
                egui::Frame::new().fill(ca(PIERRE, 0.7)).stroke(Stroke::new(1.0, c(PIERRE_CLAIRE))).inner_margin(8.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(g["nom"].as_str().unwrap_or("")).strong().color(c(OR)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Supprimer").clicked() {
                                let id = g["id"].as_str().unwrap_or("").to_string();
                                self.action("supprimer_groupe", json!({"id": id}), "Groupe supprimé (chez toi seulement).");
                            }
                        });
                    });
                    let noms: Vec<&str> = g["membres"].as_array().map(|m| m.iter().filter_map(|x| x["nom"].as_str()).collect()).unwrap_or_default();
                    ui.label(egui::RichText::new(noms.join(" · ")).color(c(LAVANDE)));
                });
            }
        });
        self.ornements(ui.painter(), rep.response.rect);
        ui.add_space(10.0);
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "NOUVEAU GROUPE");
            ui.horizontal(|ui| { ui.label("Nom :"); ui.add(egui::TextEdit::singleline(&mut self.nom_groupe).hint_text("Les explorateurs")); });
            let contacts = self.contacts();
            if contacts.is_empty() { ui.label("Aucun contact : voir l'onglet Contacts."); }
            ui.horizontal_wrapped(|ui| {
                for ct in &contacts {
                    let id = ct["id"].as_str().unwrap_or("").to_string();
                    let mut coche = self.choix_groupe.contains(&id);
                    let pastille = if ct["en_ligne"] == true { "🟢" } else { "⚪" };
                    if ui.toggle_value(&mut coche, format!("{pastille} {}", ct["nom"].as_str().unwrap_or("?"))).changed() {
                        if coche { self.choix_groupe.insert(id); } else { self.choix_groupe.remove(&id); }
                    }
                }
            });
            ui.add_space(6.0);
            if self.bouton_or(ui, "CRÉER LE GROUPE") {
                let v = json!({"nom": self.nom_groupe, "ids": self.choix_groupe.iter().collect::<Vec<_>>()});
                if self.action("creer_groupe", v, "Groupe créé et envoyé aux membres.") { self.nom_groupe.clear(); self.choix_groupe.clear(); }
            }
        });
        self.ornements(ui.painter(), rep.response.rect);
    }

    fn onglet_contacts(&mut self, ui: &mut Ui) {
        let code = self.etat["code"].as_str().unwrap_or("").to_string();
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "MON CODE D'INVITATION");
            ui.horizontal(|ui| {
                egui::Frame::new().fill(c(0xFF0E0C1A)).stroke(Stroke::new(1.0, c(OR))).inner_margin(vec2(10.0, 6.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(&code).monospace().size(17.0).color(c(OR_CLAIR)));
                });
                if ui.button("Copier").clicked() { ui.ctx().copy_text(code.clone()); self.dire("Code copié.".into()); }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Ajouter un code :");
                ui.add(egui::TextEdit::singleline(&mut self.code).hint_text("192.168.1.20:47800").desired_width(180.0));
                if ui.button("Ajouter").clicked() {
                    let r = self.appeler("ajouter", json!({"code": self.code}));
                    let ok = r["ok"] == true;
                    self.dire(if ok { format!("{} ajouté.", r["nom"].as_str().unwrap_or("?")) } else { format!("⚠ {}", r["erreur"].as_str().unwrap_or("échec")) });
                    if ok { self.code.clear(); }
                    self.rafraichir.store(true, Ordering::Relaxed);
                }
            });
            ui.small("Sur le même réseau, les PixelPal se trouvent tout seuls. Sinon (VPN, autre site), échangez vos codes.");
        });
        self.ornements(ui.painter(), rep.response.rect);
        ui.add_space(10.0);
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "EXPLORATEURS CONNUS");
            let contacts = self.contacts();
            if contacts.is_empty() { ui.label("Personne trouvé pour l'instant."); }
            for ct in contacts {
                ui.horizontal(|ui| {
                    let look = ct["look"].as_str().filter(|s| !s.is_empty()).unwrap_or("aventurier").to_string();
                    self.image_animee(ui, &look, if ct["en_ligne"] == true { "idle" } else { "sit" }, 44.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(ct["nom"].as_str().unwrap_or("?")).strong().color(c(OR)));
                        let etat = if ct["en_ligne"] == true { egui::RichText::new("● en ligne").color(c(0xFF7CE08A)) } else { egui::RichText::new("○ absent").color(c(0xFF8A82A8)) };
                        ui.horizontal(|ui| { ui.label(etat); ui.weak(format!("{}:{}", ct["host"].as_str().unwrap_or(""), ct["port"])); });
                    });
                });
            }
        });
        self.ornements(ui.painter(), rep.response.rect);
    }

    fn onglet_ecrans(&mut self, ui: &mut Ui) {
        let ecrans = self.etat["ecrans"].as_array().cloned().unwrap_or_default();
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "LA CARTE DES ÉCRANS");
            ui.label("Disposition lue dans Windows (Paramètres > Système > Affichage) :");
            let f = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0) as f32;
            if !ecrans.is_empty() {
                let (x0, y0) = ecrans.iter().fold((f32::MAX, f32::MAX), |a, e| (a.0.min(f(e, "x")), a.1.min(f(e, "y"))));
                let (x1, y1) = ecrans.iter().fold((f32::MIN, f32::MIN), |a, e| (a.0.max(f(e, "r")), a.1.max(f(e, "b"))));
                let largeur = ui.available_width();
                let k = (largeur / (x1 - x0)).min(210.0 / (y1 - y0));
                let (rect, _) = ui.allocate_exact_size(vec2(largeur, (y1 - y0) * k + 8.0), Sense::hover());
                let decal = (largeur - (x1 - x0) * k) / 2.0;
                for e in &ecrans {
                    let r = Rect::from_min_max(
                        rect.min + vec2(decal + (f(e, "x") - x0) * k + 4.0, (f(e, "y") - y0) * k + 4.0),
                        rect.min + vec2(decal + (f(e, "r") - x0) * k - 4.0, (f(e, "b") - y0) * k - 4.0));
                    let p = ui.painter();
                    p.rect_filled(r.expand(2.0), 0.0, c(NUIT));
                    p.rect_filled(r, 0.0, c(0xFF231A4A));
                    p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 3.0)), 0.0, c(PIERRE_CLAIRE));
                    let sol = rect.min.y + (f(e, "sol") - y0) * k + 4.0;
                    p.rect_filled(Rect::from_min_max(pos2(r.min.x, sol - 2.0), pos2(r.max.x, sol + 1.0)), 0.0, c(scene::FEUILLE));
                    p.rect_filled(Rect::from_min_max(pos2(r.min.x, sol + 1.0), r.max), 0.0, c(0xFF3C3656));
                    if e["principal"] == true { p.rect_stroke(r.expand(2.0), 0.0, Stroke::new(2.0, c(OR)), StrokeKind::Outside); }
                    let lab = self.etiquette(ui.ctx(), &format!("{}", e["num"]), if e["principal"] == true { OR } else { CREME });
                    let s = vec2(lab.size()[0] as f32 * 4.0, lab.size()[1] as f32 * 4.0);
                    ui.painter().image(lab.id(), Rect::from_center_size(r.center(), s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
            }
            ui.small("Bande verte : le sol sur lequel il marche (haut de la barre des tâches). Cadre doré : écran principal.");
            for l in self.etat["liens"].as_array().cloned().unwrap_or_default() {
                ui.label(egui::RichText::new(format!("» {}", l.as_str().unwrap_or(""))).color(c(LAVANDE)));
            }
            if ecrans.len() > 1 && self.etat["liens"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                ui.label("Aucun écran ne se touche : il voyagera par portail.");
            }
        });
        self.ornements(ui.painter(), rep.response.rect);
        ui.add_space(10.0);
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "DÉMO");
            ui.horizontal_wrapped(|ui| {
                if ui.button("Aller sur l'écran suivant").clicked() { self.action("demo", json!({"quoi": "ecran"}), "C'est parti !"); }
                if ui.button("Voyage par portail").clicked() { self.action("demo", json!({"quoi": "portail"}), "Abracadabra !"); }
                if ui.button("Recevoir un message test").clicked() { self.action("demo", json!({"quoi": "visiteur"}), "Un visiteur arrive (près de ta souris)."); }
            });
        });
        self.ornements(ui.painter(), rep.response.rect);
    }

    fn onglet_personnage(&mut self, ui: &mut Ui) {
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "MON EXPLORATEUR");
            ui.horizontal(|ui| {
                let look = self.look.clone();
                self.estrade(ui, &look, "wave", 120.0);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| { ui.label("Nom :"); ui.text_edit_singleline(&mut self.nom); });
                    ui.label("Tenue :");
                    ui.horizontal_wrapped(|ui| {
                        for (k, n) in sprites::LOOKS {
                            let actif = self.look == k;
                            let tex = self.texture(ui.ctx(), k, "idle", 0);
                            let b = egui::Button::image(egui::Image::new(&tex).fit_to_exact_size(vec2(44.0, 44.0))).selected(actif);
                            if ui.add(b).on_hover_text(n).clicked() { self.look = k.to_string(); }
                        }
                    });
                    ui.add_space(4.0);
                    if self.bouton_or(ui, "ENREGISTRER") {
                        let v = json!({"nom": self.nom, "look": self.look});
                        self.action("profil", v, "Profil enregistré.");
                    }
                });
            });
        });
        self.ornements(ui.painter(), rep.response.rect);
        ui.add_space(10.0);
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "STORYBOARD");
            let look = self.look.clone();
            for a in sprites::anims() {
                ui.horizontal(|ui| {
                    self.estrade(ui, &look, a.cle, 84.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(a.nom).strong().color(c(OR)));
                        ui.label(egui::RichText::new(format!("{} images · {} i/s{}", a.poses.len(), a.fps, if a.boucle { "" } else { " · une fois" })).small().color(c(LAVANDE)));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                            for i in 0..a.poses.len() {
                                let tex = self.texture(ui.ctx(), &look, a.cle, i);
                                let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                                ui.painter().rect_filled(r, 0.0, c(0xFF0E0C1A));
                                ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
        self.ornements(ui.painter(), rep.response.rect);
        ui.add_space(10.0);
        let rep = Self::cadre().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.titre(ui, "MISES À JOUR");
            ui.label(format!("Version installée : {}", self.etat["version"].as_str().unwrap_or(crate::maj::VERSION)));
            ui.label(egui::RichText::new(self.etat["maj"].as_str().unwrap_or("")).color(c(LAVANDE)));
            ui.horizontal(|ui| {
                if ui.button("Vérifier maintenant").clicked() { self.action("maj", json!({}), "Recherche lancée…"); }
                if ui.button("Quitter PixelPal").clicked() {
                    self.appeler("quitter", json!({}));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
        self.ornements(ui.painter(), rep.response.rect);
    }
}

impl eframe::App for Panneau {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let recu = self.partage.lock().unwrap().take();
        if let Some(e) = recu {
            if e["ok"] == false && e.get("erreur").is_some() {
                let m = format!("⚠ {}", e["erreur"].as_str().unwrap_or(""));
                if m != self.retour || self.retour_t.elapsed() > Duration::from_secs(8) { self.dire(m); }
            } else {
                self.etat = e;
            }
            if !self.profil_charge && self.etat["profil"].is_object() {
                self.nom = self.etat["profil"]["nom"].as_str().unwrap_or("").into();
                self.look = self.etat["profil"]["look"].as_str().unwrap_or("aventurier").into();
                self.profil_charge = true;
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(33));
        self.fond(ui);
        egui::CentralPanel::default().frame(egui::Frame::NONE.inner_margin(16.0)).show(ui, |ui| {
            self.banniere(ui);
            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 8.0);
                for (o, ico, n) in [(Onglet::Message, "lettre", "LETTRE"), (Onglet::Groupes, "groupe", "GROUPES"), (Onglet::Contacts, "lien", "CONTACTS"),
                    (Onglet::Ecrans, "ecran", "ÉCRANS"), (Onglet::Personnage, "chapeau", "PERSO")] {
                    if self.tablette(ui, ico, n, self.onglet == o) { self.onglet = o; }
                }
            });
            ui.add_space(8.0);
            if !self.retour.is_empty() && self.retour_t.elapsed() < Duration::from_secs(8) {
                let a = (1.0 - (self.retour_t.elapsed().as_secs_f32() - 6.0).max(0.0) / 2.0).clamp(0.0, 1.0);
                egui::Frame::new().fill(ca(0xFF2A1F12, 0.95 * a)).stroke(Stroke::new(1.5, ca(OR, a))).inner_margin(vec2(12.0, 6.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(format!("» {}", self.retour)).color(ca(OR_CLAIR, a)));
                });
                ui.add_space(6.0);
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(6.0);
                match self.onglet {
                    Onglet::Message => self.onglet_message(ui),
                    Onglet::Groupes => self.onglet_groupes(ui),
                    Onglet::Contacts => self.onglet_contacts(ui),
                    Onglet::Ecrans => self.onglet_ecrans(ui),
                    Onglet::Personnage => self.onglet_personnage(ui),
                }
                ui.add_space(10.0);
            });
        });
    }
}
