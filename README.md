# PixelPal

Un petit aventurier en pixel art vit sur ton bureau. Il passe d'un écran à l'autre, escalade les bords, marche au plafond et sur les fenêtres. On peut l'attraper à la souris. Il a faim, comme un Tamagotchi, et livre des lettres par portail aux collègues qui ont aussi PixelPal, même s'ils sont sur un autre réseau.

## Installer et lancer

Télécharger `pixelpal.exe` depuis la dernière [release](https://github.com/williamloree/PixelPal/releases), puis le lancer. Il se met ensuite à jour tout seul.

Au premier lancement, Windows demande l'accès au réseau : **Autoriser** (nécessaire pour les messages). Il n'y a qu'une seule instance par profil.

Pour développer :

```powershell
cargo build --release
.\target\release\pixelpal.exe              # ton personnage
.\target\release\pixelpal.exe --profil=2   # un 2e utilisateur sur le même PC, pour tester les messages
```

## Utilisation

| Geste | Effet |
| --- | --- |
| Clic sur le personnage | menu de soins : jauges ventre / santé / joie, nourrir (pomme, baie, champignon), caresser, panneau ; ressusciter s'il est mort |
| Glisser le personnage | il est tenu par le col ; lâché (ou lancé) de haut, il ouvre son parachute… ou s'écrase s'il n'en a plus |
| Clic sur un visiteur ou sa bulle | message lu : le visiteur salue et repart, l'expéditeur est prévenu |
| Clic gauche sur l'icône de notification | panneau |
| Clic droit sur l'icône de notification | « Ouvrir le panneau » / « Quitter PixelPal » |

Par défaut, il vit **derrière les fenêtres**, au niveau du bureau : une appli au premier plan le cache, et il ne vient jamais se promener sur la fenêtre active. Les visiteurs qui apportent une lettre, eux, restent toujours visibles. L'option « Rester derrière les fenêtres » se règle dans panneau > Perso.

## Ce qu'il fait

| Situation | Comportement |
| --- | --- |
| Écran voisin, même hauteur | il passe à pied |
| Écran voisin plus haut / beaucoup plus bas | il pose une échelle, grimpe ou descend, puis **range son échelle** |
| Écran voisin un peu plus bas (≤ 160 px) | il saute |
| Écran en dessous | il creuse un trou à la pelle et tombe (le trou se rebouche) |
| Écran au-dessus | il pose une échelle, grimpe, la remonte avec lui ; pour redescendre, il creuse |
| Écrans qui ne se touchent pas | il voyage par portail |
| Bord d'écran sans voisin | il l'escalade, puis marche au plafond tête en bas, et redescend (ou se laisse tomber en parachute) |
| Fenêtre ouverte (pas la fenêtre active) | il saute ou monte à l'échelle sur son dessus, s'y promène, la suit si elle bouge, tombe si elle se ferme |
| Lâché de haut | parachute s'il en a ; sinon il s'écrase, KO avec des étoiles (−15 santé) |

## Tamagotchi

- Le temps ne compte que quand l'appli est ouverte. Un repas toutes les 3 h environ. Sans manger, il a faim au bout de ~6 h, s'affaiblit, puis meurt au bout de ~12 h.
- **Réserve** : pomme (+30 ventre), baie féerique (+15 ventre, +20 joie), champignon lumineux (+45 ventre, +10 santé). Le jardin donne un aliment toutes les 40 min d'utilisation, l'atelier plie un parachute toutes les 2 h (3 au plus).
- **Mort** : il s'effondre, une tombe apparaît et son fantôme flotte au-dessus. Clic sur le fantôme (ou panneau > Soins) : « Ressusciter ». Son âge repart à zéro, son record est gardé.
- Tout est enregistré dans `%APPDATA%\PixelPal\pixelpal.json`.

## Messages : pair-à-pair avec iroh

Pas de serveur à héberger. Chaque PixelPal a un **identifiant public fixe** (sa clé dans `%APPDATA%\PixelPal\cle.bin`) : c'est son **code d'invitation** (52 caractères, onglet Contacts).

- La connexion se fait avec [iroh](https://www.iroh.computer/) : chiffrée de bout en bout, elle perce les box (NAT) pour aller en direct. Quand c'est impossible, elle passe par les relais publics de n0, qui ne peuvent pas lire le contenu. Le code marche partout : télétravail, 4G, autre site.
- Sur le même réseau, les PixelPal se trouvent aussi tout seuls (annonce UDP, port 47899, et mDNS).
- Onglet Groupes : on crée un groupe, qui est envoyé à tous ses membres. Onglet Lettre : « Tout le groupe » ou « Seulement… ». Le personnage ouvre un portail, la lettre part chez tous **en même temps**, et chez chacun un visiteur sort d'un portail près de la souris.
- L'identité de l'expéditeur vient de la connexion chiffrée : personne ne peut se faire passer pour un autre.

## Releases et mise à jour automatique

- **Workflow** `.github/workflows/release.yml` : à chaque push sur `main` (hors fichiers `.md`), il lance les tests, compile pour Windows, puis publie une release `vX.Y.N` avec `pixelpal.exe`. `X.Y` vient de `Cargo.toml`, et `N` est le numéro d'exécution du workflow.
- **Mise à jour** (`src/maj.rs`) : l'exécutable publié vérifie la dernière release au démarrage, toutes les 6 heures, ou via panneau > Perso > « Vérifier maintenant ». Il télécharge, remplace son exécutable, annonce la nouvelle version et redémarre. Une version compilée à la main ne se met jamais à jour.
- **Dépôt privé** : GitHub refuse alors le téléchargement sans authentification. Il faut un jeton (« fine-grained », lecture seule, *Contents: Read*) dans `%APPDATA%\PixelPal\github-token.txt` ou dans la variable `PIXELPAL_GITHUB_TOKEN`.

## Le personnage et le panneau

Chaque image est une **pose** (angles des membres, objet tenu, yeux, bouche…) dessinée sur une toile de 40 × 40 pixels, puis détourée automatiquement calque par calque. Il y a 21 animations, dont la marche, l'escalade, le creusement, le portail, le parachute, le repas, la joie et le KO. Les tenues sont des variantes de palette ; tout est dans `src/sprites.rs`.

Le panneau (egui) a pour thème une nuit au jardin féerique. En bannière, une scène calme où il attend devant sa maisonnette (fumée de cheminée, lanterne) avec un château sur la colline. Les titres sont dans une police pixel maison, les boutons en Segoe UI. Onglets : Lettre, Soins, Groupes, Contacts, Écrans, Perso (tenue, comportement, storyboard animé, mises à jour).

- Planche PNG des animations : `pixelpal.exe --planche planche.png [--look bleu]`.
- Aperçu de la bannière : `pixelpal.exe --scene scene.png`.

## Code

| Fichier | Rôle |
| --- | --- |
| `src/main.rs` | boucle principale (30 images/s), prise à la souris, menu, commandes du panneau, envois |
| `src/monde.rs` | simulation : écrans, trajets, surfaces (sol, fenêtres, plafond), chutes, soins, visiteurs (+ tests) |
| `src/soins.rs` | Tamagotchi : faim, santé, joie, réserve, jardin, mort (+ tests) |
| `src/menu.rs` | menu de soins en pixel art |
| `src/sprites.rs` | personnage par poses ; échelle, trou, portail, parachute, tombe, cœurs |
| `src/fenetres.rs` | fenêtres Windows transparentes, ordre d'empilement, fenêtres praticables, écrans, icône |
| `src/reseau.rs`, `src/pair.rs` | pair-à-pair iroh, découverte locale, canal local du panneau |
| `src/panneau.rs`, `src/scene.rs` | panneau egui, bannière, police pixel, icônes |
| `src/maj.rs` | mise à jour automatique depuis les releases GitHub |
| `src/config.rs` | profil, contacts, groupes, soins, options |
| `tools/*.ps1` | outils de test : piloter une instance (`ctl.ps1`, avec `PIXELPAL_JETON`), cliquer, lâcher, ouvrir le menu, capturer |

Tests : `cargo test` (trajets entre écrans, échelles rangées, chutes avec et sans parachute, fenêtres, escalade, repas, mort et résurrection).
