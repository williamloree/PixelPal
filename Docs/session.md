# Session de création de PixelPal (19 et 20 novembre 2026)

Archive de la session de travail qui a produit PixelPal, de l'idée à la version 0.2.5. Dépôt : https://github.com/williamloree/PixelPal

## Demande de départ

Un petit personnage en pixel art qui se promène sur l'écran et passe d'un écran à l'autre selon la disposition de Windows (« Personnaliser l'emplacement des écrans ») :

- échelle pour monter, trou pour descendre ;
- animations détaillées, avec un storyboard ;
- groupes entre utilisateurs de l'appli, et lettres livrées par portail, à tout le groupe en même temps ;
- personnage générique au départ, modifiable ensuite.

## Chronologie

| Étape | Ce qui a été fait |
| --- | --- |
| 1. Premiers choix | Personnage « aventurier », projet dans `C:\Dev\Perso\PixelPal`, liaison directe entre utilisateurs (« tunnel »), sans serveur. |
| 2. Ébauche Electron | Commencée puis abandonnée à la demande : passage à **Rust**. |
| 3. V1 Rust | Fenêtres Windows transparentes « layered », personnage dessiné par poses (13 animations), trajets entre écrans (marche, saut, échelle, trou, portail), messages en TCP direct avec découverte UDP, panneau egui. Premier push sur GitHub. |
| 4. Releases | Workflow GitHub (tests, compilation, release `v0.1.N` à chaque push sur `main`), mise à jour automatique dans l'appli, panneau refait au thème « temple au jardin féerique ». |
| 5. Corrections | Icône de notification à la tête de l'aventurier, clic droit avec « Quitter », une seule instance par profil, panneau qui ne gèle plus, tests rendus déterministes. |
| 6. Version 0.2 | Réseau **iroh** (hors réseau local), **Tamagotchi**, prise à la souris (parachute ou écrasement), escalade des bords, marche au plafond et sur les fenêtres, vie derrière les fenêtres, échelles rangées après usage, bannière calme (maisonnette, château), boutons en Segoe UI, onglet Soins. |
| 7. Présentation | README sur le modèle de ClaudeIsles, images générées par `pixelpal.exe --vitrine assets/images`. |

## Décisions prises avec William

| Sujet | Décision |
| --- | --- |
| Langage | Rust : un seul exécutable, pas d'Electron. |
| Personnage | Aventurier (chapeau, sac, écharpe), 5 tenues en variantes de palette. |
| Réseau hors réseau local | iroh : pair-à-pair chiffré, perçage NAT, relais publics de n0 en secours, code d'invitation fixe. Pas de serveur à héberger. |
| Mort de faim | Tombe et fantôme, puis résurrection (âge remis à zéro, record gardé). |
| Rythme de la faim | Lent, et seulement quand l'appli est ouverte : un repas toutes les 3 h environ, mort après ~12 h sans manger. |
| Grimper | Les trois : bords des écrans (puis plafond), dessus des fenêtres, et accès à l'écran du dessus. |
| Place à l'écran | Derrière les fenêtres, au niveau du bureau ; jamais sur la fenêtre active. Option pour le remettre devant. Les visiteurs avec une lettre restent toujours visibles. |
| Échelles | Rangées une fois utilisées ; une nouvelle pour redescendre (ou un trou). |
| Bannière du panneau | Calme : il attend devant sa maisonnette, château sur la colline, peu d'animation. |
| Police des boutons | Segoe UI Semibold (la police pixel agrandie « faisait pâté ») ; police pixel gardée pour les titres. |

## Architecture

| Fichier | Rôle |
| --- | --- |
| `src/main.rs` | Boucle principale (30 images/s), prise à la souris, menu, commandes du panneau, envois. |
| `src/monde.rs` | Simulation : écrans et liens, trajets, surfaces (sol, fenêtres, plafond), chutes, soins, visiteurs. |
| `src/soins.rs` | Tamagotchi : faim, santé, joie, réserve, jardin, parachutes, mort. |
| `src/menu.rs` | Menu de soins en pixel art. |
| `src/sprites.rs` | Personnage par poses (21 animations) ; échelle, trou, portail, parachute, tombe, cœurs. |
| `src/toile.rs` | Toile de pixels : collage, détourage, rotation, retournement, fantôme. |
| `src/fenetres.rs` | Fenêtres transparentes, ordre d'empilement, fenêtres praticables, écrans, icône, verrou d'instance. |
| `src/reseau.rs`, `src/pair.rs` | iroh, découverte locale (UDP + mDNS), canal local du panneau sur 127.0.0.1. |
| `src/panneau.rs`, `src/scene.rs` | Panneau egui, bannière, police pixel maison, icônes, dalles. |
| `src/maj.rs` | Mise à jour automatique depuis les releases GitHub. |
| `src/config.rs` | Profil, contacts, groupes, soins, options : `%APPDATA%\PixelPal\pixelpal.json` (clé iroh : `cle.bin`). |
| `src/vitrine.rs` | Images du README. |
| `.github/workflows/release.yml` | Tests, compilation, release `vX.Y.N`. |
| `tools/*.ps1` | Outils de test (piloter une instance avec `PIXELPAL_JETON`, cliquer, lâcher, menu, capture). |

## Vérifications faites

- **Tests automatiques** : 15 tests (`cargo test`), stables sur 30 à 40 exécutions d'affilée. Ils couvrent :
  - les trajets entre écrans et l'échelle rangée ;
  - le trou pour redescendre et le portail ;
  - la chute avec parachute et l'écrasement sans parachute ;
  - la fenêtre qui bouge puis se ferme ;
  - l'escalade, le repas, la mort et la résurrection ;
  - la livraison d'une lettre.
- **En vrai**, deux instances sur le même PC, avec clics simulés et captures d'écran :
  - réseau : ajout par code, groupe et lettre via iroh, en liaison directe ;
  - écran : visiteur au premier plan, personnage au-dessus du bureau mais sous les applis ;
  - gestes : prise au col, parachute, KO avec étoiles, marche au plafond, menu de soins et repas ;
  - panneau : thème sombre, onglet Soins.
- **Sur GitHub** : compilations n° 2 à 5 réussies. La n° 1 a échoué (notes de release sur plusieurs lignes), puis a été corrigée.

## Versions publiées

| Version | Contenu |
| --- | --- |
| v0.1.2 | Workflow, mise à jour automatique, panneau « temple ». |
| v0.1.3 | Icône de l'aventurier, menu au clic droit. |
| v0.2.4 | iroh, Tamagotchi, prise à la souris, escalade, vie derrière les fenêtres. |
| v0.2.5 | README et images (28,4 Mo, contre 14,9 Mo en 0.1.3 à cause d'iroh). |

## Points ouverts

- **Plusieurs écrans** : jamais testé en vrai (un seul écran branché pendant la session). La cause probable du « il ne monte jamais sur l'écran du dessus » est corrigée : il ne visitait que les écrans reliés au sien, et la détection tolère maintenant un décalage. À vérifier au bureau ; en cas de souci, une capture de l'onglet Écrans aidera.
- **iroh entre deux box différentes** : non testé, les deux instances étaient sur le même PC. Le relais de secours a été vu en action par l'essai préalable.
- **Dépôt public** : la mise à jour automatique marche sans jeton. S'il passe en privé, il faudra un jeton en lecture sur chaque poste, ou un second dépôt public pour les exécutables seuls.
- **Taille de l'exe** : réglages possibles (`lto`, `codegen-units = 1`, `panic = "abort"`) pour descendre vers ~15 Mo, avec une compilation environ deux fois plus longue.
- **Workflow** : un push qui ne touche que `assets/` relance une compilation. L'exclusion n'a pas été ajoutée (modification refusée pendant la session).
- **Pare-feu** : l'essai d'iroh a laissé deux règles « iroh-essai Inbound Block », sans effet sur PixelPal. Pour les retirer, en administrateur : `Remove-NetFirewallRule -DisplayName iroh-essai`.
- **Idées évoquées** : envoyer de la nourriture à un collègue par portail ; personnage à l'image de William ou mascotte Styx.
