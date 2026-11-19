//! Réseau pair-à-pair avec iroh : connexions chiffrées qui traversent les box (perçage NAT),
//! avec les relais publics de n0 en secours. Chaque PixelPal a un identifiant public fixe
//! (sa clé, gardée dans cle.bin) : c'est son « code d'invitation », valable partout.
//! Sur le réseau local, les PixelPal s'annoncent aussi en UDP (port 47899) pour se trouver tout seuls.
//! Le panneau parle au programme principal en TCP sur 127.0.0.1 (commandes « ctl » avec jeton).

use crate::pair;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::Path;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PORT_TCP: u16 = 47800;
const PORT_UDP: u16 = 47899;

#[derive(Clone, Debug)]
pub struct Identite { pub id: String, pub nom: String, pub look: String }

#[derive(Clone, Debug)]
pub struct Contact { pub id: String, pub nom: String, pub look: String, pub vu: Option<Instant>, pub chemin: String }

impl Contact {
    pub fn en_ligne(&self) -> bool { self.vu.map(|v| v.elapsed() < Duration::from_secs(75)).unwrap_or(false) }
}

pub enum Evt {
    Message(Value),
    Groupe(Value),
    Lu(Value),
    Controle(Value, Sender<Value>),
    Contacts,
}

pub struct Reseau {
    pub port: u16, // port local du panneau
    pub moi: Mutex<Identite>,
    pub contacts: Mutex<HashMap<String, Contact>>,
    iroh: pair::Reseau,
    jeton: String,
    tx: Mutex<Sender<Evt>>,
}

/// Envoie une ligne JSON en TCP et lit la réponse (canal local du panneau).
pub fn requete(host: &str, port: u16, v: &Value, delai: Duration) -> Result<Value, String> {
    let adr: SocketAddr = format!("{host}:{port}").parse().map_err(|_| format!("adresse invalide : {host}:{port}"))?;
    let mut s = TcpStream::connect_timeout(&adr, delai).map_err(|e| format!("{host}:{port} injoignable ({e})"))?;
    s.set_read_timeout(Some(Duration::from_secs(25))).ok();
    s.write_all(format!("{v}\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut l = String::new();
    BufReader::new(s.take(1 << 20)).read_line(&mut l).map_err(|e| e.to_string())?;
    serde_json::from_str(l.trim()).map_err(|_| "réponse illisible".to_string())
}

fn ip_locale() -> Ipv4Addr {
    UdpSocket::bind("0.0.0.0:0").and_then(|s| { s.connect("8.8.8.8:80")?; s.local_addr() })
        .ok().and_then(|a| match a.ip() { std::net::IpAddr::V4(v) => Some(v), _ => None }).unwrap_or(Ipv4Addr::LOCALHOST)
}

/// Vrai si la chaîne est un code d'invitation iroh valide.
pub fn code_valide(code: &str) -> bool { pair::lire_code(code).is_ok() }

impl Reseau {
    /// Démarre iroh (clé persistante dans `dossier`), la découverte locale et le canal du panneau.
    /// Renvoie aussi notre code d'invitation.
    pub fn demarrer(dossier: &Path, mut moi: Identite, connus: &[crate::config::Membre], jeton: String, tx: Sender<Evt>) -> Result<Arc<Reseau>, String> {
        let cle = pair::charger_ou_creer_cle(&dossier.join("cle.bin")).map_err(|e| e.to_string())?;
        let (iroh, entrantes) = pair::Reseau::demarrer(cle).map_err(|e| format!("démarrage du réseau : {e}"))?;
        moi.id = pair::code_invitation(&iroh.id());
        let ecoute = (PORT_TCP..PORT_TCP + 40).find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok()).ok_or("aucun port local libre")?;
        let port = ecoute.local_addr().map(|a| a.port()).unwrap_or(PORT_TCP);
        let contacts = connus.iter().filter(|m| code_valide(&m.id))
            .map(|m| (m.id.clone(), Contact { id: m.id.clone(), nom: m.nom.clone(), look: "aventurier".into(), vu: None, chemin: String::new() })).collect();
        let r = Arc::new(Reseau { port, moi: Mutex::new(moi), contacts: Mutex::new(contacts), iroh, jeton, tx: Mutex::new(tx) });

        // requêtes des autres PixelPal (arrivent par iroh)
        let r2 = r.clone();
        std::thread::spawn(move || for req in entrantes {
            let de = pair::code_invitation(&req.de);
            let rep = r2.recevoir(&de, &req.json);
            req.repondre(rep.to_string());
        });
        // commandes du panneau (local uniquement)
        let r3 = r.clone();
        std::thread::spawn(move || for s in ecoute.incoming().flatten() {
            let r4 = r3.clone();
            std::thread::spawn(move || r4.connexion_locale(s));
        });
        r.udp();
        // de temps en temps, on prend des nouvelles des contacts (pour savoir qui est en ligne)
        let r5 = r.clone();
        std::thread::spawn(move || loop {
            let a_tester: Vec<String> = r5.contacts.lock().unwrap().values()
                .filter(|c| c.vu.map(|v| v.elapsed() > Duration::from_secs(45)).unwrap_or(true)).map(|c| c.id.clone()).collect();
            for id in a_tester {
                let r6 = r5.clone();
                std::thread::spawn(move || { let _ = r6.envoyer_a(&id, json!({"type": "ping"})); });
            }
            std::thread::sleep(Duration::from_secs(30));
        });
        Ok(r)
    }

    pub fn code(&self) -> String { self.moi.lock().unwrap().id.clone() }

    pub fn identite(&self) -> Value {
        let m = self.moi.lock().unwrap();
        json!({"id": m.id, "name": m.nom, "look": m.look})
    }

    fn signaler(&self, e: Evt) { let _ = self.tx.lock().unwrap().send(e); }

    fn maj_contact(&self, id: &str, nom: &str, look: &str) {
        if id == self.code() || !code_valide(id) { return; }
        let chemin = self.iroh.chemin(id).unwrap_or_default();
        let mut c = self.contacts.lock().unwrap();
        let e = c.entry(id.to_string()).or_insert(Contact { id: id.into(), nom: String::new(), look: String::new(), vu: None, chemin: String::new() });
        if !nom.is_empty() && nom != "?" { e.nom = nom.chars().take(40).collect(); }
        if !look.is_empty() { e.look = look.chars().take(20).collect(); }
        // chemin utilisé par iroh : direct (perçage réussi) ou par un relais
        if let Some(utilise) = chemin.split(" | ").find(|p| p.contains("(utilisé)")) {
            e.chemin = if utilise.starts_with("DIRECT") { "direct".into() } else { "relais".into() };
        }
        e.vu = Some(Instant::now());
        drop(c);
        self.signaler(Evt::Contacts);
    }

    /// Envoie un message signé de notre identité à un autre PixelPal (par son code), et lit sa réponse.
    /// Bloquant (jusqu'à 20 s si le pair est injoignable) : à appeler depuis un fil à part.
    pub fn envoyer_a(&self, id: &str, mut v: Value) -> Result<Value, String> {
        v["app"] = json!("pixelpal");
        v["from"] = self.identite();
        let texte = self.iroh.requete(id, &v.to_string()).map_err(|e| format!("{e:#}"))?;
        let rep: Value = serde_json::from_str(&texte).map_err(|_| "réponse illisible".to_string())?;
        self.maj_contact(id, rep["from"]["name"].as_str().unwrap_or(""), rep["from"]["look"].as_str().unwrap_or(""));
        Ok(rep)
    }

    /// Ajout manuel par code d'invitation.
    pub fn ajouter_par_code(&self, code: &str) -> Result<String, String> {
        let code = code.trim();
        if !code_valide(code) { return Err("code d'invitation invalide (52 caractères, à copier depuis l'onglet Contacts de ton collègue)".into()); }
        if code == self.code() { return Err("c'est ton propre code !".into()); }
        let rep = self.envoyer_a(code, json!({"type": "hello"}))?;
        rep["from"]["name"].as_str().map(String::from).ok_or_else(|| "ce n'est pas un PixelPal".into())
    }

    /// Message reçu par iroh ; `de` est l'identifiant authentifié de l'expéditeur.
    fn recevoir(&self, de: &str, texte: &str) -> Value {
        let Ok(mut v) = serde_json::from_str::<Value>(texte) else { return json!({"ok": false}) };
        if v["app"] != "pixelpal" { return json!({"ok": false}); }
        // l'identité vient de la connexion chiffrée : impossible de se faire passer pour quelqu'un d'autre
        v["from"]["id"] = json!(de);
        self.maj_contact(de, v["from"]["name"].as_str().unwrap_or("?"), v["from"]["look"].as_str().unwrap_or(""));
        match v["type"].as_str().unwrap_or("") {
            "hello" | "ping" => json!({"ok": true, "from": self.identite()}),
            "groupe" => { self.signaler(Evt::Groupe(v)); json!({"ok": true, "from": self.identite()}) }
            "msg" => { self.signaler(Evt::Message(v)); json!({"ok": true, "from": self.identite()}) }
            "lu" => { self.signaler(Evt::Lu(v)); json!({"ok": true}) }
            _ => json!({"ok": false}),
        }
    }

    fn connexion_locale(&self, s: TcpStream) {
        s.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut l = String::new();
        let Ok(mut lecteur) = s.try_clone() else { return };
        if BufReader::new((&mut lecteur).take(1 << 20)).read_line(&mut l).is_err() { return; }
        let Ok(v) = serde_json::from_str::<Value>(l.trim()) else { return };
        let rep = if v["type"] != "ctl" || v["jeton"].as_str() != Some(self.jeton.as_str()) {
            json!({"ok": false, "erreur": "refusé"})
        } else if v["cmd"] == "ajouter" {
            match self.ajouter_par_code(v["code"].as_str().unwrap_or("")) {
                Ok(n) => json!({"ok": true, "nom": n}),
                Err(e) => json!({"ok": false, "erreur": e}),
            }
        } else {
            let (t, r) = channel();
            self.signaler(Evt::Controle(v, t));
            r.recv_timeout(Duration::from_secs(4)).unwrap_or(json!({"ok": false, "erreur": "pas de réponse"}))
        };
        let mut s = s;
        let _ = s.write_all(format!("{rep}\n").as_bytes());
    }

    /// Découverte sur le réseau local : chacun annonce son code, son nom et sa tenue.
    fn udp(self: &Arc<Self>) {
        use socket2::{Domain, Protocol, Socket, Type};
        let Ok(s) = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)) else { return };
        let _ = s.set_reuse_address(true);
        let _ = s.set_broadcast(true);
        if s.bind(&SocketAddr::from(([0, 0, 0, 0], PORT_UDP)).into()).is_err() { return; }
        let u: UdpSocket = s.into();
        let Ok(u2) = u.try_clone() else { return };
        let r = self.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 2048];
            while let Ok((n, _)) = u.recv_from(&mut buf) {
                let Ok(v) = serde_json::from_slice::<Value>(&buf[..n]) else { continue };
                if v["app"] != "pixelpal" || v["v"] != 2 { continue; }
                let id = v["id"].as_str().unwrap_or("");
                // on ne fait confiance à l'annonce que pour découvrir le code ; le nom sera confirmé par iroh
                let nouveau = !r.contacts.lock().unwrap().contains_key(id);
                r.maj_contact(id, v["name"].as_str().unwrap_or("?"), v["look"].as_str().unwrap_or(""));
                if nouveau {
                    let (r2, id2) = (r.clone(), id.to_string());
                    std::thread::spawn(move || { let _ = r2.envoyer_a(&id2, json!({"type": "hello"})); });
                }
            }
        });
        let r = self.clone();
        std::thread::spawn(move || loop {
            let mut v = r.identite();
            v["app"] = json!("pixelpal");
            v["v"] = json!(2);
            let octets = v.to_string();
            let ip = ip_locale().octets();
            for dest in [Ipv4Addr::BROADCAST, Ipv4Addr::new(ip[0], ip[1], ip[2], 255)] {
                let _ = u2.send_to(octets.as_bytes(), (dest, PORT_UDP));
            }
            std::thread::sleep(Duration::from_secs(5));
        });
    }
}
