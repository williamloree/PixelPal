//! Panneau (fenêtre egui, processus séparé) : messages, groupes, contacts, écrans, personnage et storyboard.
//! Il pilote le programme principal par des commandes « ctl » en local.

use crate::reseau::requete;
use crate::sprites::{self, Sprites};
use eframe::egui;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub fn lancer(port: u16, jeton: String) -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 720.0]).with_title("PixelPal"),
        ..Default::default()
    };
    eframe::run_native("PixelPal", opts, Box::new(move |_cc| Ok(Box::new(Panneau::new(port, jeton)))))
}

#[derive(PartialEq, Clone, Copy)]
enum Onglet { Message, Groupes, Contacts, Ecrans, Personnage }

struct Panneau {
    port: u16,
    jeton: String,
    etat: Value,
    maj: Instant,
    onglet: Onglet,
    retour: String,
    // message
    groupe: String,
    tous: bool,
    choix: HashSet<String>,
    texte: String,
    // groupes
    nom_groupe: String,
    choix_groupe: HashSet<String>,
    // contacts
    code: String,
    // personnage
    nom: String,
    look: String,
    profil_charge: bool,
    sprites: Sprites,
    textures: HashMap<(String, &'static str, usize), egui::TextureHandle>,
    debut: Instant,
}

impl Panneau {
    fn new(port: u16, jeton: String) -> Self {
        Panneau { port, jeton, etat: Value::Null, maj: Instant::now() - Duration::from_secs(10), onglet: Onglet::Message, retour: String::new(),
            groupe: String::new(), tous: true, choix: HashSet::new(), texte: String::new(), nom_groupe: String::new(), choix_groupe: HashSet::new(),
            code: String::new(), nom: String::new(), look: String::new(), profil_charge: false, sprites: Sprites::default(), textures: HashMap::new(),
            debut: Instant::now() }
    }

    fn appeler(&self, cmd: &str, mut extra: Value) -> Value {
        if !extra.is_object() { extra = json!({}); }
        extra["type"] = json!("ctl");
        extra["jeton"] = json!(self.jeton);
        extra["cmd"] = json!(cmd);
        requete("127.0.0.1", self.port, &extra, Duration::from_secs(2)).unwrap_or_else(|e| json!({"ok": false, "erreur": format!("PixelPal ne répond pas ({e})")}))
    }

    /// Lance une commande et garde le message de retour.
    fn action(&mut self, cmd: &str, extra: Value, succes: &str) -> bool {
        let r = self.appeler(cmd, extra);
        let ok = r["ok"] == true;
        self.retour = if ok { succes.to_string() } else { format!("⚠ {}", r["erreur"].as_str().unwrap_or("échec")) };
        self.maj = Instant::now() - Duration::from_secs(10);
        ok
    }

    fn texture(&mut self, ctx: &egui::Context, look: &str, anim: &'static str, i: usize) -> egui::TextureHandle {
        let cle = (look.to_string(), anim, i);
        if let Some(t) = self.textures.get(&cle) { return t.clone(); }
        let img = self.sprites.image_n(look, anim, i);
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.w as usize, img.h as usize], &img.vers_rgba());
        let t = ctx.load_texture(format!("{look}-{anim}-{i}"), ci, egui::TextureOptions::NEAREST);
        self.textures.insert(cle, t.clone());
        t
    }

    fn image_animee(&mut self, ui: &mut egui::Ui, look: &str, anim: &'static str, taille: f32) {
        let t = self.debut.elapsed().as_secs_f32();
        let i = sprites::index_image(anim, t % 3.0);
        let tex = self.texture(ui.ctx(), look, anim, i);
        ui.add(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(taille, taille)));
    }

    fn contacts(&self) -> Vec<Value> { self.etat["contacts"].as_array().cloned().unwrap_or_default() }
    fn en_ligne(&self, id: &str) -> Option<bool> { self.contacts().iter().find(|c| c["id"] == id).map(|c| c["en_ligne"] == true) }

    fn onglet_message(&mut self, ui: &mut egui::Ui) {
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
                for x in &groupes {
                    ui.selectable_value(&mut self.groupe, x["id"].as_str().unwrap_or("").to_string(), x["nom"].as_str().unwrap_or(""));
                }
            });
        });
        let moi = self.etat["profil"]["id"].as_str().unwrap_or("").to_string();
        let membres: Vec<Value> = g["membres"].as_array().cloned().unwrap_or_default().into_iter().filter(|m| m["id"] != moi.as_str()).collect();
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.tous, true, "Tout le groupe");
            ui.radio_value(&mut self.tous, false, "Seulement…");
        });
        if !self.tous {
            for m in &membres {
                let id = m["id"].as_str().unwrap_or("").to_string();
                let mut c = self.choix.contains(&id);
                let pastille = match self.en_ligne(&id) { Some(true) => "🟢", _ => "⚪" };
                if ui.checkbox(&mut c, format!("{pastille} {}", m["nom"].as_str().unwrap_or("?"))).changed() {
                    if c { self.choix.insert(id); } else { self.choix.remove(&id); }
                }
            }
        }
        ui.add(egui::TextEdit::multiline(&mut self.texte).hint_text("Ton message…").desired_rows(5).desired_width(f32::INFINITY));
        if ui.button("✉  Envoyer par portail").clicked() {
            let a = if self.tous { Value::Null } else { json!(self.choix.iter().collect::<Vec<_>>()) };
            let v = json!({"groupe": self.groupe, "a": a, "texte": self.texte});
            if self.action("envoyer", v, "Ton personnage part livrer la lettre !") { self.texte.clear(); }
        }
        ui.small("Il ouvre un portail, y entre, et la lettre arrive chez chaque destinataire en même temps. Un visiteur sort d'un portail sur leur écran.");
    }

    fn onglet_groupes(&mut self, ui: &mut egui::Ui) {
        let groupes = self.etat["groupes"].as_array().cloned().unwrap_or_default();
        if groupes.is_empty() { ui.label("Aucun groupe pour l'instant."); }
        for g in &groupes {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(g["nom"].as_str().unwrap_or(""));
                    if ui.small_button("Supprimer").clicked() {
                        let id = g["id"].as_str().unwrap_or("").to_string();
                        self.action("supprimer_groupe", json!({"id": id}), "Groupe supprimé (chez toi seulement).");
                    }
                });
                let noms: Vec<&str> = g["membres"].as_array().map(|m| m.iter().filter_map(|x| x["nom"].as_str()).collect()).unwrap_or_default();
                ui.label(noms.join(", "));
            });
        }
        ui.separator();
        ui.heading("Nouveau groupe");
        ui.horizontal(|ui| { ui.label("Nom :"); ui.text_edit_singleline(&mut self.nom_groupe); });
        let contacts = self.contacts();
        if contacts.is_empty() { ui.label("Aucun contact : voir l'onglet Contacts."); }
        for c in &contacts {
            let id = c["id"].as_str().unwrap_or("").to_string();
            let mut coche = self.choix_groupe.contains(&id);
            let pastille = if c["en_ligne"] == true { "🟢" } else { "⚪" };
            if ui.checkbox(&mut coche, format!("{pastille} {}", c["nom"].as_str().unwrap_or("?"))).changed() {
                if coche { self.choix_groupe.insert(id); } else { self.choix_groupe.remove(&id); }
            }
        }
        if ui.button("Créer le groupe").clicked() {
            let v = json!({"nom": self.nom_groupe, "ids": self.choix_groupe.iter().collect::<Vec<_>>()});
            if self.action("creer_groupe", v, "Groupe créé et envoyé aux membres.") { self.nom_groupe.clear(); self.choix_groupe.clear(); }
        }
    }

    fn onglet_contacts(&mut self, ui: &mut egui::Ui) {
        let code = self.etat["code"].as_str().unwrap_or("").to_string();
        ui.horizontal(|ui| {
            ui.label("Ton code d'invitation :");
            ui.monospace(&code);
            if ui.small_button("Copier").clicked() { ui.ctx().copy_text(code.clone()); self.retour = "Code copié.".into(); }
        });
        ui.horizontal(|ui| {
            ui.label("Ajouter un code :");
            ui.add(egui::TextEdit::singleline(&mut self.code).hint_text("192.168.1.20:47800").desired_width(180.0));
            if ui.button("Ajouter").clicked() {
                let r = self.appeler("ajouter", json!({"code": self.code}));
                self.retour = if r["ok"] == true { format!("{} ajouté.", r["nom"].as_str().unwrap_or("?")) } else { format!("⚠ {}", r["erreur"].as_str().unwrap_or("échec")) };
                if r["ok"] == true { self.code.clear(); }
                self.maj = Instant::now() - Duration::from_secs(10);
            }
        });
        ui.small("Sur le même réseau, les PixelPal se trouvent tout seuls. Sinon (VPN, autre site), échangez vos codes.");
        ui.separator();
        let contacts = self.contacts();
        if contacts.is_empty() { ui.label("Personne trouvé pour l'instant."); }
        for c in contacts {
            ui.horizontal(|ui| {
                ui.label(if c["en_ligne"] == true { "🟢" } else { "⚪" });
                ui.strong(c["nom"].as_str().unwrap_or("?"));
                ui.weak(format!("{}:{}", c["host"].as_str().unwrap_or(""), c["port"]));
            });
        }
    }

    fn onglet_ecrans(&mut self, ui: &mut egui::Ui) {
        let ecrans = self.etat["ecrans"].as_array().cloned().unwrap_or_default();
        ui.label("Disposition lue dans Windows (Paramètres > Système > Affichage) :");
        let f = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0) as f32;
        if !ecrans.is_empty() {
            let (x0, y0) = ecrans.iter().fold((f32::MAX, f32::MAX), |a, e| (a.0.min(f(e, "x")), a.1.min(f(e, "y"))));
            let (x1, y1) = ecrans.iter().fold((f32::MIN, f32::MIN), |a, e| (a.0.max(f(e, "r")), a.1.max(f(e, "b"))));
            let largeur = ui.available_width().min(500.0);
            let k = (largeur / (x1 - x0)).min(200.0 / (y1 - y0));
            let (rect, _) = ui.allocate_exact_size(egui::vec2(largeur, (y1 - y0) * k + 4.0), egui::Sense::hover());
            let p = ui.painter();
            for e in &ecrans {
                let r = egui::Rect::from_min_max(
                    rect.min + egui::vec2((f(e, "x") - x0) * k + 2.0, (f(e, "y") - y0) * k + 2.0),
                    rect.min + egui::vec2((f(e, "r") - x0) * k - 2.0, (f(e, "b") - y0) * k - 2.0));
                p.rect_filled(r, 4.0, egui::Color32::from_rgb(52, 58, 74));
                let sol = rect.min.y + (f(e, "sol") - y0) * k;
                p.line_segment([egui::pos2(r.min.x, sol), egui::pos2(r.max.x, sol)], egui::Stroke::new(2.0, egui::Color32::from_rgb(196, 160, 90)));
                let txt = format!("{}{}", e["num"], if e["principal"] == true { " ★" } else { "" });
                p.text(r.center(), egui::Align2::CENTER_CENTER, txt, egui::FontId::proportional(22.0), egui::Color32::WHITE);
            }
        }
        ui.small("Trait doré : le sol sur lequel il marche (haut de la barre des tâches). ★ : écran principal.");
        for l in self.etat["liens"].as_array().cloned().unwrap_or_default() { ui.label(format!("• {}", l.as_str().unwrap_or(""))); }
        if ecrans.len() > 1 && self.etat["liens"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
            ui.label("Aucun écran ne se touche : il voyagera par portail.");
        }
        ui.separator();
        ui.heading("Démo");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Aller sur l'écran suivant").clicked() { self.action("demo", json!({"quoi": "ecran"}), "C'est parti !"); }
            if ui.button("Voyage par portail").clicked() { self.action("demo", json!({"quoi": "portail"}), "Abracadabra !"); }
            if ui.button("Recevoir un message test").clicked() { self.action("demo", json!({"quoi": "visiteur"}), "Un visiteur arrive (près de ta souris)."); }
        });
    }

    fn onglet_personnage(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let look = self.look.clone();
            self.image_animee(ui, &look, "idle", 96.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| { ui.label("Nom :"); ui.text_edit_singleline(&mut self.nom); });
                ui.horizontal(|ui| {
                    ui.label("Tenue :");
                    let nom_look = sprites::LOOKS.iter().find(|l| l.0 == self.look).map(|l| l.1).unwrap_or("?");
                    egui::ComboBox::from_id_salt("look").selected_text(nom_look).show_ui(ui, |ui| {
                        for (k, n) in sprites::LOOKS { ui.selectable_value(&mut self.look, k.to_string(), n); }
                    });
                });
                if ui.button("Enregistrer").clicked() {
                    let v = json!({"nom": self.nom, "look": self.look});
                    self.action("profil", v, "Profil enregistré.");
                }
            });
        });
        ui.separator();
        ui.heading("Storyboard des animations");
        let look = self.look.clone();
        for a in sprites::anims() {
            ui.horizontal(|ui| {
                self.image_animee(ui, &look, a.cle, 80.0);
                ui.vertical(|ui| {
                    ui.strong(format!("{}  ·  {} images, {} i/s{}", a.nom, a.poses.len(), a.fps, if a.boucle { "" } else { ", une fois" }));
                    ui.horizontal_wrapped(|ui| {
                        for i in 0..a.poses.len() {
                            let tex = self.texture(ui.ctx(), &look, a.cle, i);
                            ui.add(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(48.0, 48.0)));
                        }
                    });
                });
            });
        }
        ui.separator();
        if ui.button("Quitter PixelPal").clicked() {
            self.appeler("quitter", json!({}));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for Panneau {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.maj.elapsed() > Duration::from_millis(1500) {
            self.maj = Instant::now();
            let e = self.appeler("etat", json!({}));
            if e["ok"] == false && e.get("erreur").is_some() { self.retour = format!("⚠ {}", e["erreur"].as_str().unwrap_or("")); } else { self.etat = e; }
            if !self.profil_charge && self.etat["profil"].is_object() {
                self.nom = self.etat["profil"]["nom"].as_str().unwrap_or("").into();
                self.look = self.etat["profil"]["look"].as_str().unwrap_or("aventurier").into();
                self.profil_charge = true;
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                let look = self.look.clone();
                self.image_animee(ui, &look, "walk", 40.0);
                ui.heading("PixelPal");
                if let Some(p) = self.etat["profil_arg"].as_str() { ui.weak(format!("(profil de test {p})")); }
            });
            ui.horizontal(|ui| {
                for (o, n) in [(Onglet::Message, "✉ Message"), (Onglet::Groupes, "Groupes"), (Onglet::Contacts, "Contacts"), (Onglet::Ecrans, "Écrans"), (Onglet::Personnage, "Personnage")] {
                    ui.selectable_value(&mut self.onglet, o, n);
                }
            });
            if !self.retour.is_empty() { ui.colored_label(egui::Color32::from_rgb(220, 190, 120), &self.retour); }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match self.onglet {
                Onglet::Message => self.onglet_message(ui),
                Onglet::Groupes => self.onglet_groupes(ui),
                Onglet::Contacts => self.onglet_contacts(ui),
                Onglet::Ecrans => self.onglet_ecrans(ui),
                Onglet::Personnage => self.onglet_personnage(ui),
            });
        });
    }
}
