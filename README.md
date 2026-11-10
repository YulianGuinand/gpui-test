# my_gpui_mobile — messagerie GPUI Kit sur Android

Démo mobile de [GPUI Kit](https://gpui-kit.com) : liste de conversations et
écran de messagerie virtualisé, alimenté par un jeu de données mock massif
(10k / 50k / 100k messages) pour observer les performances sur émulateur.

## Stack

| Partie | Choix |
| --- | --- |
| UI | `gpui-kit` rev `ae6a3fdf` (couche `component` uniquement, pas d'`assets`) |
| GPUI | `gpui-pre 0.3.10` (snapshot aligné sur Kit et sur le host mobile) |
| Host mobile | `gpui-pre-mobile` (fork `longbridge/gpui-mobile` rev `7548ebc`) — `NativeActivity` `dev.gpui.mobile.GpuiActivity` |
| Build Android | `cargo-ndk` → `.so` dans `jniLibs` → Gradle (AGP 8.13) → `adb` |
| ABI | `x86_64` (émulateur par défaut), `arm64-v8a` pour téléphone |

## Prérequis

- Rust (cibles `x86_64-linux-android`, `aarch64-linux-android`)
- `cargo install cargo-ndk`
- Android SDK + NDK (`%LOCALAPPDATA%\Android\Sdk`), un émulateur ou un appareil connecté (`adb devices`)
- Java 17+ : le script utilise automatiquement le JBR d'Android Studio si `JAVA_HOME` n'est pas défini

## Construire, installer, lancer

```powershell
.\scripts\build-android.ps1                 # debug x86_64 + install + lancement
.\scripts\build-android.ps1 -Release        # release (chiffres de perf réels)
.\scripts\build-android.ps1 -Abi arm64-v8a  # téléphone physique
.\scripts\build-android.ps1 -NoRun          # build uniquement
.\scripts\build-android.ps1 -Check          # cargo check rapide
```

## Observations perf

```powershell
adb logcat -s gpui-msg:D
```

- `perf [chat 3] fps=59.4 build=0.83ms rows=14 window=2003ms` — FPS mesuré sur
  fenêtre de 2 s, temps de construction de l'arbre d'éléments, **nombre de
  rangées réellement construites** (virtualisation : rester ~10-20 même avec
  20 000 messages dans le fil).
- `dataset 50k — 50000 messages … generated in N ms` — coût de génération.
- Boutons `10k / 50k / 100k` en haut de la liste : changer d'échelle à la volée.
- Bouton `HUD` dans l'en-tête du chat : masquer/afficher le bandeau.
- Le HUD vert en haut à droite reprend les mêmes métriques en direct.

## Architecture

```
src/
├── lib.rs            # entry android_main (gpui-mobile) + ouverture de la fenêtre
├── app.rs            # AppView : liste ⇄ chat, changement d'échelle, HUD
├── mock.rs           # générateur déterministe (PRNG xorshift, std only)
├── perf.rs           # compteurs FPS / temps de build / rangées virtualisées
└── screens/chat.rs   # MessageScroller + Message/Bubble/Avatar + composer + streaming simulé
android/              # hôte Gradle (NativeActivity, manifest, ressources du fork)
scripts/build-android.ps1
```

## Données mock

- 50 conversations (dont des groupes), dont une thread « stress » = 40 % du jeu.
- PRNG seedé → jeu reproductible, mesures comparables.
- Horodatages répartis sur 90 jours, marqueurs « Aujourd'hui / Hier / lun. 3 »,
  états Delivered / Read, compteurs de non-lus, séparateur « Non lu ».
- Les messages sont partagés en `Arc<Vec<Message>>` : ouverture d'un fil sans
  copie ; les envois et les réponses streamées atterrissent dans un petit
  tampon `extra` (append + `remeasure_items` = cas de perf concret).

## Limites connues (doc GPUI Kit « Mobile »)

- Support mobile **expérimental** : la doc officielle ne couvre que l'iOS ;
  le chemin Android est celui de l'exemple du fork (validé ici sur émulateur).
- Pas de feature `assets` : les icônes nommées Kit sont indisponibles, l'UI
  utilise textes/avatars/bulles.
- Les chiffres de perf « réels » exigent `-Release` sur appareil physique.
