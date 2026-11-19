//! Réseau pair-à-pair : chaque PixelPal écoute en TCP (port 47800 et suivants) et s'annonce en UDP
//! sur le réseau local (port 47899). Un message = une ligne JSON, une réponse = une ligne JSON.
//! Le panneau parle au programme principal par le même canal (commandes « ctl », locales et avec jeton).

use crate::config::Membre;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PORT_TCP: u16 = 47800;
const PORT_UDP: u16 = 47899;

#[derive(Clone, Debug)]
pub struct Identite { pub id: String, pub nom: String, pub look: String }

#[derive(Clone, Debug)]
pub struct Contact { pub id: String, pub nom: String, pub host: String, pub port: u16, pub look: String, pub vu: Option<Instant> }

impl Contact {
    pub fn en_ligne(&self) -> bool { self.vu.map(|v| v.elapsed() < Duration::from_secs(15)).unwrap_or(false) }
}

pub enum Evt {
    Message(Value),
    Groupe(Value),
    Lu(Value),
    Controle(Value, Sender<Value>),
    Contacts,
}

pub struct Reseau {
    pub port: u16,
    pub moi: Mutex<Identite>,
    pub contacts: Mutex<HashMap<String, Contact>>,
    jeton: String,
    tx: Mutex<Sender<Evt>>,
}

pub fn ip_locale() -> Ipv4Addr {
    UdpSocket::bind("0.0.0.0:0").and_then(|s| { s.connect("8.8.8.8:80")?; s.local_addr() })
        .ok().and_then(|a| match a.ip() { IpAddr::V4(v) => Some(v), _ => None }).unwrap_or(Ipv4Addr::LOCALHOST)
}

/// Envoie une ligne JSON et lit la réponse.
pub fn requete(host: &str, port: u16, v: &Value, delai: Duration) -> Result<Value, String> {
    let adr: SocketAddr = format!("{host}:{port}").parse().map_err(|_| format!("adresse invalide : {host}:{port}"))?;
    let mut s = TcpStream::connect_timeout(&adr, delai).map_err(|e| format!("{host}:{port} injoignable ({e})"))?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
    s.write_all(format!("{v}\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut l = String::new();
    BufReader::new(s.take(1 << 20)).read_line(&mut l).map_err(|e| e.to_string())?;
    serde_json::from_str(l.trim()).map_err(|_| "réponse illisible".to_string())
}

impl Reseau {
    pub fn demarrer(moi: Identite, connus: &[Membre], jeton: String, tx: Sender<Evt>, garder_port: bool) -> Arc<Reseau> {
        // après une mise à jour, l'ancienne version libère le port dans quelques instants : on l'attend
        let mut premier = None;
        for _ in 0..if garder_port { 40 } else { 1 } {
            if let Ok(l) = TcpListener::bind(("0.0.0.0", PORT_TCP)) { premier = Some((l, PORT_TCP)); break; }
            std::thread::sleep(Duration::from_millis(200));
        }
        let (ecoute, port) = premier.or_else(|| (PORT_TCP + 1..PORT_TCP + 40)
            .find_map(|p| TcpListener::bind(("0.0.0.0", p)).ok().map(|l| (l, p))))
            .expect("aucun port TCP libre");
        let contacts = connus.iter().map(|m| (m.id.clone(), Contact { id: m.id.clone(), nom: m.nom.clone(), host: m.host.clone(), port: m.port, look: "aventurier".into(), vu: None })).collect();
        let r = Arc::new(Reseau { port, moi: Mutex::new(moi), contacts: Mutex::new(contacts), jeton, tx: Mutex::new(tx) });

        let r2 = r.clone();
        std::thread::spawn(move || {
            for s in ecoute.incoming().flatten() {
                let r3 = r2.clone();
                std::thread::spawn(move || r3.connexion(s));
            }
        });
        r.udp();
        let r4 = r.clone();
        std::thread::spawn(move || loop {
            // on vérifie de temps en temps les contacts qui ne s'annoncent pas (autre réseau, VPN)
            std::thread::sleep(Duration::from_secs(15));
            let a_tester: Vec<Contact> = r4.contacts.lock().unwrap().values()
                .filter(|c| !c.host.is_empty() && c.vu.map(|v| v.elapsed() > Duration::from_secs(12)).unwrap_or(true)).cloned().collect();
            for c in a_tester {
                let r5 = r4.clone();
                std::thread::spawn(move || { let _ = r5.envoyer_a(&c.host, c.port, json!({"type": "ping"}), Duration::from_secs(2)); });
            }
        });
        r
    }

    pub fn identite(&self) -> Value {
        let m = self.moi.lock().unwrap();
        json!({"id": m.id, "name": m.nom, "look": m.look, "port": self.port})
    }

    fn signaler(&self, e: Evt) { let _ = self.tx.lock().unwrap().send(e); }

    fn maj_contact(&self, id: &str, nom: &str, host: &str, port: u16, look: &str) {
        if id == self.moi.lock().unwrap().id || id.is_empty() { return; }
        let mut c = self.contacts.lock().unwrap();
        let e = c.entry(id.to_string()).or_insert(Contact { id: id.into(), nom: String::new(), host: String::new(), port: 0, look: String::new(), vu: None });
        e.nom = nom.chars().take(40).collect();
        e.host = host.into();
        e.port = port;
        if !look.is_empty() { e.look = look.chars().take(20).collect(); }
        e.vu = Some(Instant::now());
        drop(c);
        self.signaler(Evt::Contacts);
    }

    /// Envoie un message signé de notre identité à un autre PixelPal.
    pub fn envoyer_a(&self, host: &str, port: u16, mut v: Value, delai: Duration) -> Result<Value, String> {
        v["app"] = json!("pixelpal");
        v["from"] = self.identite();
        let rep = requete(host, port, &v, delai)?;
        if let Some(f) = rep.get("from") {
            self.maj_contact(f["id"].as_str().unwrap_or(""), f["name"].as_str().unwrap_or("?"), host, port, f["look"].as_str().unwrap_or(""));
        } else if let Some(id) = v.get("vers").and_then(|x| x.as_str()) {
            if let Some(c) = self.contacts.lock().unwrap().get_mut(id) { c.vu = Some(Instant::now()); }
        }
        Ok(rep)
    }

    /// Adresse à jour d'un membre (le contact découvert est prioritaire sur l'adresse enregistrée).
    pub fn adresse(&self, m: &Membre) -> (String, u16) {
        match self.contacts.lock().unwrap().get(&m.id) {
            Some(c) if !c.host.is_empty() => (c.host.clone(), c.port),
            _ => (m.host.clone(), m.port),
        }
    }

    /// Ajout manuel par « adresse:port » (le code d'invitation).
    pub fn ajouter_par_code(&self, code: &str) -> Result<String, String> {
        let code = code.trim();
        let (h, p) = code.rsplit_once(':').ok_or("code attendu : adresse:port, par exemple 192.168.1.20:47800")?;
        let p: u16 = p.trim().parse().map_err(|_| "port invalide")?;
        let rep = self.envoyer_a(h.trim(), p, json!({"type": "hello"}), Duration::from_secs(3))?;
        let nom = rep["from"]["name"].as_str().ok_or("ce n'est pas un PixelPal")?.to_string();
        Ok(nom)
    }

    fn connexion(&self, s: TcpStream) {
        s.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let host = s.peer_addr().map(|a| a.ip().to_canonical().to_string()).unwrap_or_default();
        let mut l = String::new();
        let Ok(mut lecteur) = s.try_clone() else { return };
        if BufReader::new((&mut lecteur).take(1 << 20)).read_line(&mut l).is_err() { return; }
        let Ok(v) = serde_json::from_str::<Value>(l.trim()) else { return };
        let rep = self.traiter(v, &host);
        let mut s = s;
        let _ = s.write_all(format!("{rep}\n").as_bytes());
    }

    fn traiter(&self, v: Value, host: &str) -> Value {
        if v["type"] == "ctl" {
            // commande du panneau : seulement en local, avec le jeton
            if host != "127.0.0.1" || v["jeton"].as_str() != Some(self.jeton.as_str()) { return json!({"ok": false, "erreur": "refusé"}); }
            if v["cmd"] == "ajouter" {
                return match self.ajouter_par_code(v["code"].as_str().unwrap_or("")) {
                    Ok(n) => json!({"ok": true, "nom": n}),
                    Err(e) => json!({"ok": false, "erreur": e}),
                };
            }
            let (t, r) = channel();
            self.signaler(Evt::Controle(v, t));
            return r.recv_timeout(Duration::from_secs(4)).unwrap_or(json!({"ok": false, "erreur": "pas de réponse"}));
        }
        if v["app"] != "pixelpal" { return json!({"ok": false}); }
        let f = &v["from"];
        let Some(id) = f["id"].as_str() else { return json!({"ok": false}) };
        self.maj_contact(id, f["name"].as_str().unwrap_or("?"), host, f["port"].as_u64().unwrap_or(0) as u16, f["look"].as_str().unwrap_or(""));
        match v["type"].as_str().unwrap_or("") {
            "hello" | "ping" => json!({"ok": true, "from": self.identite()}),
            "groupe" => { self.signaler(Evt::Groupe(v)); json!({"ok": true}) }
            "msg" => { self.signaler(Evt::Message(v)); json!({"ok": true}) }
            "lu" => { self.signaler(Evt::Lu(v)); json!({"ok": true}) }
            _ => json!({"ok": false}),
        }
    }

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
            while let Ok((n, src)) = u.recv_from(&mut buf) {
                let Ok(v) = serde_json::from_slice::<Value>(&buf[..n]) else { continue };
                if v["app"] != "pixelpal" { continue; }
                let host = src.ip().to_canonical().to_string();
                r.maj_contact(v["id"].as_str().unwrap_or(""), v["name"].as_str().unwrap_or("?"), &host, v["port"].as_u64().unwrap_or(0) as u16, v["look"].as_str().unwrap_or(""));
            }
        });
        let r = self.clone();
        std::thread::spawn(move || loop {
            let mut v = r.identite();
            v["app"] = json!("pixelpal");
            let octets = v.to_string();
            let ip = ip_locale().octets();
            for dest in [Ipv4Addr::BROADCAST, Ipv4Addr::new(ip[0], ip[1], ip[2], 255)] {
                let _ = u2.send_to(octets.as_bytes(), (dest, PORT_UDP));
            }
            std::thread::sleep(Duration::from_secs(4));
        });
    }
}
