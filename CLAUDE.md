# PixelPal — consignes pour Claude

## Git : pousser directement sur `main` après chaque fonctionnalité

- Travailler directement sur `main` : **pas de branche, pas de pull request**.
- Dès qu'une fonctionnalité (ou une correction) est terminée et vérifiée, **commiter et pousser sur `main` sans demander de confirmation** :
  `git push origin main`.
  Chaque push sur `main` lance `.github/workflows/release.yml`, qui teste, compile et publie automatiquement une nouvelle release (`vX.Y.N`). Les PixelPal installés se mettent ensuite à jour tout seuls.
- **Ne pas compiler ni lancer de tests en local** : ni `cargo build`, ni `cargo test`, ni lancement de l'appli. L'utilisateur teste lui-même, et la CI lance `cargo test` avant chaque release.
  Il faut donc relire soigneusement le code avant de pousser (types, emprunts, `match` exhaustifs, tests existants à adapter) : une erreur de compilation fait échouer la release.
- Les tests unitaires restent à écrire ou à mettre à jour dans le code ([src/monde.rs](src/monde.rs), `mod tests`), même si on ne les lance pas.
- Un commit par fonctionnalité. Le message est en français, au même style que l'historique (ex. « Nouvelles actions : nettoyage d'écran, jumelles, piolet »).
- Ne jamais forcer le push (`--force`) et ne jamais réécrire l'historique de `main`.

## Projet

- Rust 2024, egui, iroh ; Windows uniquement. Le code, les commentaires, les noms et les textes affichés sont **en français**.
- Le personnage est fait de poses dessinées par code ([src/sprites.rs](src/sprites.rs)). Ses comportements sont des files d'`Action` ([src/monde.rs](src/monde.rs)), et le rendu dans les fenêtres Windows est dans [src/main.rs](src/main.rs).
- Outils de l'utilisateur (à ne pas lancer soi-même) : `pixelpal.exe --planche planche.png` exporte toutes les animations, `pixelpal.exe --vitrine assets/images` régénère les images du README.
- Une nouvelle animation ou action visible doit aussi être ajoutée :
  - au README, section « Ce que tu vois » ;
  - aux boutons « DÉMO » du panneau ([src/panneau.rs](src/panneau.rs)), si elle se déclenche à la demande.
