# PixelPal

Un petit aventurier en pixel art se promène sur la barre des tâches, passe d'un écran à l'autre et livre des messages par portail aux collègues qui ont aussi PixelPal.

## Lancer

```powershell
cargo build --release
.\target\release\pixelpal.exe              # ton personnage
.\target\release\pixelpal.exe --profil=2   # un 2e utilisateur sur le même PC, pour tester les messages
```

Au premier lancement, Windows demande l'accès au réseau : **Autoriser** (nécessaire pour les messages). Une seule instance par profil : relancer l'exécutable ne crée pas de deuxième personnage.

## Releases et mise à jour automatique

- **Workflow** `.github/workflows/release.yml` : à chaque push sur `main` (hors fichiers `.md`), tests, compilation Windows, puis release `v0.1.N` avec `pixelpal.exe` en pièce jointe. `N` = numéro d'exécution du workflow : rien à incrémenter à la main. Pour passer en 0.2, changer `version` dans `Cargo.toml`.
- **Mise à jour** (`src/maj.rs`) : l'exécutable publié vérifie la dernière release au démarrage, toutes les 6 heures, ou via panneau > Perso > « Vérifier maintenant ». S'il y a plus récent, il télécharge, remplace son exécutable, annonce « Nouvelle version… » et redémarre tout seul. Une version compilée à la main ne se met jamais à jour.
- **Dépôt privé** : GitHub refuse alors le téléchargement sans authentification. Il faut un jeton (« fine-grained », lecture seule sur ce dépôt, permission *Contents: Read*) dans `%APPDATA%\PixelPal\github-token.txt` ou la variable `PIXELPAL_GITHUB_TOKEN`, sur chaque poste.

- **Clic sur le personnage** (ou sur l'icône de la zone de notification) : ouvre le panneau.
- **Clic sur un visiteur ou sa bulle** : marque le message comme lu ; le visiteur salue et repart par son portail, l'expéditeur est prévenu.
- **Quitter** : panneau > Personnage > « Quitter PixelPal ».

## Ce qu'il fait

| Situation (disposition lue dans Windows) | Animation |
| --- | --- |
| Écran voisin, même hauteur | il passe à pied |
| Écran voisin plus haut | il pose une échelle (marteau) et grimpe |
| Écran voisin un peu plus bas (≤ 160 px) | il saute |
| Écran voisin beaucoup plus bas | il pose une échelle depuis le haut et descend |
| Écran en dessous | il creuse un trou à la pelle et tombe |
| Écran au-dessus | il pose une échelle et grimpe |
| Échelle ou trou déjà là (90 s) | il le réutilise (une échelle sert dans les deux sens) |
| Écrans qui ne se touchent pas | il voyage par portail |

Le reste du temps il se promène, s'assoit, salue, regarde autour de lui.

## Messages : le « tunnel » pair-à-pair

Pas de serveur : chaque PixelPal écoute en TCP (port 47800, ou le suivant libre) et s'annonce en UDP sur le réseau local (port 47899).

1. Sur le même réseau, les PixelPal se trouvent tout seuls (onglet Contacts). Sinon (VPN, autre site), on échange son **code d'invitation** (`adresse:port`).
2. Onglet Groupes : on crée un groupe ; il est envoyé à tous ses membres.
3. Onglet Message : « Tout le groupe » ou « Seulement… ». Le personnage lève la lettre, ouvre un portail et y entre. La lettre part alors chez tous les destinataires **en même temps** ; chez chacun, un visiteur (avec la tenue de l'expéditeur) sort d'un portail près de la souris et présente la lettre. Le personnage revient ensuite et dit à qui la lettre a été livrée.

Limite actuelle : entre deux réseaux séparés par une box (internet), la connexion directe ne passe pas sans redirection de port. Il faudra un petit relais ou du « hole punching » pour ça.

## Le personnage

Chaque image est une **pose** (angles des membres, objet tenu, yeux, bouche…) dessinée sur une toile de 40 × 40 pixels, puis détourée automatiquement calque par calque (dos, corps, bras avant). 13 animations : repos, marche, marche avec lettre, saut, réception, chute, escalade, creuser, poser une échelle, ouvrir un portail, présenter la lettre, saluer, assis. Les autres tenues sont des variantes de palette.

- Storyboard animé : panneau > Perso.
- Planche PNG : `pixelpal.exe --planche planche.png [--look bleu]`.
- Aperçu de la scène du panneau : `pixelpal.exe --scene scene.png`.

## Le panneau

Thème « temple au jardin féerique » : bannière animée en pixel art (le personnage court sur un chemin de pierre et saute les rochers, façon Temple Run, vers un temple aux runes lumineuses ; lune, aurore, lianes, champignons géants, fée, lucioles), dalles moussues en fond, onglets en tablettes de pierre, titres en police pixel maison (`src/scene.rs`), cadres ornés de feuillage, boutons dorés.
- Modifier le personnage : `src/sprites.rs` (`palette`, `tete`, `chapeau`, `corps`, `sac`, et les poses dans `anims()`).

## Code

| Fichier | Rôle |
| --- | --- |
| `src/main.rs` | boucle principale (30 images/s), commandes du panneau, envois |
| `src/monde.rs` | simulation : écrans, liens, trajets, actions, visiteurs (+ tests : `cargo test`) |
| `src/sprites.rs` | personnage par poses, échelle, trou, portail |
| `src/fenetres.rs` | fenêtres Windows transparentes, bulles, écrans, icône |
| `src/reseau.rs` | pair-à-pair TCP + découverte UDP |
| `src/panneau.rs` | panneau egui (processus séparé) |
| `src/scene.rs` | scène animée du panneau, police pixel, icônes, dalles |
| `src/maj.rs` | mise à jour automatique depuis les releases GitHub |
| `.github/workflows/release.yml` | compilation et publication des releases |
| `src/config.rs` | profil, contacts, groupes : `%APPDATA%\PixelPal\pixelpal.json` |
| `tools/clic-perso.ps1` | test : clique sur le personnage et capture l'écran |
