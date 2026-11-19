//! Couche réseau pair-à-pair au-dessus d'iroh 1.5, utilisable depuis du code SYNCHRONE.
//!
//! - Un fil dédié possède le runtime tokio (multi-thread) et l'endpoint iroh.
//! - `Reseau::requete(id, json)` est bloquant : utilisable depuis n'importe quel fil
//!   non-tokio (boucle Win32, fil de travail…), via `Handle::block_on`.
//! - Les requêtes reçues arrivent sur un `std::sync::mpsc::Receiver<RequeteEntrante>` ;
//!   le code synchrone répond avec `RequeteEntrante::repondre(json)`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use iroh::endpoint::{Connection, presets};
use iroh::{Endpoint, EndpointId, SecretKey};
use iroh_mdns_address_lookup::MdnsAddressLookup;
use tokio::runtime::Handle;
use tokio::sync::oneshot;

/// Protocole applicatif négocié dans la poignée de main QUIC (ALPN).
pub const ALPN: &[u8] = b"pixelpal/1";
/// Taille maximale d'un message (requête ou réponse).
const TAILLE_MAX: usize = 64 * 1024;
/// Délai maximal pour une requête complète (connexion + envoi + réponse).
const DELAI_REQUETE: Duration = Duration::from_secs(20);
/// Délai laissé au code synchrone pour répondre à une requête entrante.
const DELAI_REPONSE_LOCALE: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Clé persistante et code d'invitation
// ---------------------------------------------------------------------------

/// Charge la clé secrète depuis `chemin`, ou la génère et l'enregistre (32 octets bruts).
pub fn charger_ou_creer_cle(chemin: &Path) -> Result<SecretKey> {
    if let Ok(octets) = std::fs::read(chemin) {
        let tableau: [u8; 32] = octets
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("{} : clé corrompue ({} octets)", chemin.display(), octets.len()))?;
        return Ok(SecretKey::from_bytes(&tableau));
    }
    let cle = SecretKey::generate();
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(chemin, cle.to_bytes())
        .with_context(|| format!("écriture de {}", chemin.display()))?;
    Ok(cle)
}

/// Identifiant public -> « code d'invitation » (z-base-32, 52 caractères).
/// `id.to_string()` donne aussi une forme hexadécimale (64 caractères).
pub fn code_invitation(id: &EndpointId) -> String {
    id.to_z32()
}

/// Code d'invitation (z-base-32 OU hexadécimal) -> identifiant public.
/// Attention : `str::parse::<EndpointId>()` accepte l'hexa et le base32 RFC 4648,
/// mais PAS le z-base-32 de `to_z32()` : il faut `EndpointId::from_z32`.
pub fn lire_code(code: &str) -> Result<EndpointId> {
    let code = code.trim();
    EndpointId::from_z32(code)
        .or_else(|_| code.parse::<EndpointId>())
        .map_err(|e| anyhow!("code d'invitation invalide : {e}"))
}

// ---------------------------------------------------------------------------
// Requêtes entrantes, vues du code synchrone
// ---------------------------------------------------------------------------

/// Une requête reçue d'un pair, à traiter par le code synchrone.
pub struct RequeteEntrante {
    pub de: EndpointId,
    pub json: String,
    reponse: oneshot::Sender<String>,
}

impl RequeteEntrante {
    /// Envoie la réponse au pair (appel non bloquant, utilisable hors tokio).
    pub fn repondre(self, json: String) {
        let _ = self.reponse.send(json);
    }
}

// ---------------------------------------------------------------------------
// Le service réseau
// ---------------------------------------------------------------------------

pub struct Reseau {
    handle: Handle,
    endpoint: Endpoint,
    connexions: Arc<Mutex<HashMap<EndpointId, Connection>>>,
    arret: Option<oneshot::Sender<()>>,
    fil: Option<std::thread::JoinHandle<()>>,
}

impl Reseau {
    /// Démarre le fil réseau. Bloque jusqu'à ce que l'endpoint soit créé.
    /// Renvoie aussi le récepteur des requêtes entrantes.
    pub fn demarrer(cle: SecretKey) -> Result<(Reseau, mpsc::Receiver<RequeteEntrante>)> {
        let (tx_pret, rx_pret) = mpsc::channel::<Result<(Handle, Endpoint)>>();
        let (tx_arret, rx_arret) = oneshot::channel::<()>();
        let (tx_requetes, rx_requetes) = mpsc::channel::<RequeteEntrante>();

        let fil = std::thread::Builder::new()
            .name("reseau-iroh".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx_pret.send(Err(e.into()));
                        return;
                    }
                };
                rt.block_on(async move {
                    let endpoint = match creer_endpoint(cle).await {
                        Ok(ep) => ep,
                        Err(e) => {
                            let _ = tx_pret.send(Err(e));
                            return;
                        }
                    };
                    let _ = tx_pret.send(Ok((Handle::current(), endpoint.clone())));

                    // Boucle d'acceptation, jusqu'à la demande d'arrêt.
                    tokio::select! {
                        _ = boucle_serveur(endpoint.clone(), tx_requetes) => {}
                        _ = rx_arret => {}
                    }
                    // Fermeture propre : prévient les pairs connectés.
                    endpoint.close().await;
                });
            })?;

        let (handle, endpoint) = rx_pret
            .recv()
            .map_err(|_| anyhow!("le fil réseau s'est arrêté au démarrage"))??;
        Ok((
            Reseau {
                handle,
                endpoint,
                connexions: Arc::default(),
                arret: Some(tx_arret),
                fil: Some(fil),
            },
            rx_requetes,
        ))
    }

    /// Notre identifiant public.
    pub fn id(&self) -> EndpointId {
        self.endpoint.id()
    }

    /// Requête synchrone : envoie `json` au pair `id` (code d'invitation) et renvoie sa réponse.
    /// À appeler depuis un fil NON tokio (sinon `block_on` panique).
    pub fn requete(&self, id: &str, json: &str) -> Result<String> {
        let id = lire_code(id)?;
        let ep = self.endpoint.clone();
        let cache = self.connexions.clone();
        let json = json.to_owned();
        self.handle.block_on(async move {
            tokio::time::timeout(DELAI_REQUETE, requete_async(&ep, &cache, id, json.as_bytes()))
                .await
                .map_err(|_| anyhow!("délai dépassé ({DELAI_REQUETE:?})"))?
        })
    }

    /// Décrit le chemin réseau actuellement utilisé vers `id` (direct ou relais).
    pub fn chemin(&self, id: &str) -> Result<String> {
        let id = lire_code(id)?;
        let conn = self.connexions.lock().unwrap().get(&id).cloned();
        Ok(match conn {
            Some(c) => decrire_chemin(&c),
            None => "pas de connexion ouverte".into(),
        })
    }
}

impl Drop for Reseau {
    fn drop(&mut self) {
        if let Some(arret) = self.arret.take() {
            let _ = arret.send(());
        }
        if let Some(fil) = self.fil.take() {
            let _ = fil.join();
        }
    }
}

/// Crée l'endpoint : relais n0 + découverte DNS/pkarr n0 (preset N0) + mDNS local.
async fn creer_endpoint(cle: SecretKey) -> Result<Endpoint> {
    let mut builder = Endpoint::builder(presets::N0)
        .secret_key(cle)
        .alpns(vec![ALPN.to_vec()]);
    // Découverte sur le réseau local (crate iroh-mdns-address-lookup).
    // PIXELPAL_SANS_MDNS=1 : pour tester la découverte n0 (DNS/pkarr) seule.
    if std::env::var_os("PIXELPAL_SANS_MDNS").is_none() {
        builder = builder.address_lookup(MdnsAddressLookup::builder());
    }
    Ok(builder.bind().await?)
}

/// Accepte les connexions ; chaque connexion peut porter plusieurs flux (une requête par flux).
async fn boucle_serveur(endpoint: Endpoint, tx: mpsc::Sender<RequeteEntrante>) {
    while let Some(entrant) = endpoint.accept().await {
        let tx = tx.clone();
        tokio::spawn(async move {
            let conn = match entrant.await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[reseau] connexion entrante refusée : {e:#}");
                    return;
                }
            };
            let de = conn.remote_id();
            loop {
                let (mut envoi, mut reception) = match conn.accept_bi().await {
                    Ok(flux) => flux,
                    Err(_) => break, // connexion fermée par le pair
                };
                let tx = tx.clone();
                tokio::spawn(async move {
                    let requete = match reception.read_to_end(TAILLE_MAX).await {
                        Ok(o) => String::from_utf8_lossy(&o).into_owned(),
                        Err(e) => {
                            eprintln!("[reseau] lecture : {e:#}");
                            return;
                        }
                    };
                    let (tx_rep, rx_rep) = oneshot::channel();
                    let reponse = if tx
                        .send(RequeteEntrante { de, json: requete, reponse: tx_rep })
                        .is_err()
                    {
                        r#"{"erreur":"service arrêté"}"#.to_string()
                    } else {
                        match tokio::time::timeout(DELAI_REPONSE_LOCALE, rx_rep).await {
                            Ok(Ok(r)) => r,
                            _ => r#"{"erreur":"pas de réponse"}"#.to_string(),
                        }
                    };
                    if envoi.write_all(reponse.as_bytes()).await.is_ok() {
                        let _ = envoi.finish();
                    }
                });
            }
        });
    }
}

async fn requete_async(
    ep: &Endpoint,
    cache: &Mutex<HashMap<EndpointId, Connection>>,
    id: EndpointId,
    json: &[u8],
) -> Result<String> {
    // Réutilise la connexion si elle est encore ouverte, sinon on se reconnecte.
    let existante = cache.lock().unwrap().get(&id).cloned();
    let conn = match existante {
        Some(c) if c.close_reason().is_none() => c,
        _ => {
            // Connexion par le SEUL identifiant : l'adresse est trouvée par la découverte.
            let c = ep.connect(id, ALPN).await.context("connexion au pair")?;
            cache.lock().unwrap().insert(id, c.clone());
            c
        }
    };
    let (mut envoi, mut reception) = conn.open_bi().await.context("ouverture du flux")?;
    envoi.write_all(json).await.context("envoi")?;
    envoi.finish().context("fin d'envoi")?;
    let reponse = reception.read_to_end(TAILLE_MAX).await.context("lecture de la réponse")?;
    if reponse.is_empty() {
        bail!("réponse vide");
    }
    Ok(String::from_utf8(reponse)?)
}

fn decrire_chemin(conn: &Connection) -> String {
    let chemins = conn.paths();
    let mut morceaux = Vec::new();
    for p in chemins.iter() {
        let genre = if p.is_ip() { "DIRECT" } else if p.is_relay() { "RELAIS" } else { "autre" };
        let choisi = if p.is_selected() { " (utilisé)" } else { "" };
        morceaux.push(format!("{genre} {:?} rtt={:?}{choisi}", p.remote_addr(), p.rtt()));
    }
    if morceaux.is_empty() { "aucun chemin".into() } else { morceaux.join(" | ") }
}
