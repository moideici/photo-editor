# Prisme — plan complet

Éditeur d'images hybride **bitmap + vectoriel (SVG)** pour Windows, écrit de zéro.
Inspiré du périmètre fonctionnel de PhotoFiltre 11, sans reprendre son code, son binaire ni ses ressources.
Licence : MIT (libre, aucune clé, aucun essai).

## Architecture

```
┌────────────────────────── Interface (React + TypeScript, WebView2) ──────────────────────────┐
│ Menus · barre d'outils · panneaux (calques, historique, propriétés, couleurs) · canvas       │
│ Outils interactifs : pinceau, gomme, sélection, déplacement, formes, texte (aperçu local)     │
└───────────────▲─────────────────────────── IPC Tauri (JSON + octets bruts) ───────────────────┘
                │
┌───────────────┴──────────────────────── Cœur Rust (src-tauri) ────────────────────────────────┐
│ document/  Document, calques (Raster | Vecteur), sélection (masque), métadonnées               │
│ history/   Annuler/rétablir par instantanés partagés (Arc) : seuls les calques modifiés coûtent │
│ raster/    Filtres et réglages parallélisés (rayon), remplissage, dégradés, transformations      │
│ vector/    Formes éditables → SVG ; import SVG (usvg) ; rastérisation (resvg/tiny-skia)          │
│ io/        PNG, JPEG, WebP, BMP, GIF, TIFF, ICO, SVG, projet .prisme (zip : JSON + PNG)         │
│ interop/   Formats PhotoFiltre (.pfs sélections, .pfv tracés) ; hôte plugins .pfl (phase 4)     │
└───────────────────────────────────────────────────────────────────────────────────────────────┘
```

Pourquoi un cœur hybride : un bitmap est une grille de pixels, un SVG est une description de formes.
Les calques vectoriels restent des formes jusqu'à l'export ; resvg les convertit en pixels seulement pour l'aplatissement et l'export bitmap.
Un export SVG garde le vectoriel tel quel, et les calques bitmap y sont intégrés en `<image>`.

## Phase 1 — Version 1 utilisable (implémentée)

- [x] Fichiers : ouvrir PNG/JPEG/WebP/BMP/GIF/TIFF/ICO/SVG, glisser-déposer, enregistrer le projet `.prisme`, exporter PNG/JPEG (qualité)/WebP/BMP/TIFF/SVG
- [x] Nouvelle image (taille, fond), coller depuis le presse-papiers
- [x] Calques bitmap et vectoriels : ajouter, dupliquer, supprimer, monter/descendre, visibilité, opacité, modes de fusion, renommer, fusionner vers le bas, aplatir
- [x] Annuler/rétablir illimité (borné en mémoire), panneau historique
- [x] Sélection rectangle / ellipse / tout / aucune / inverser ; import d'une sélection PhotoFiltre `.pfs`
- [x] Outils : déplacement, pinceau, gomme, pot de peinture (tolérance), pipette, dégradé, texte, rectangle, ellipse, ligne, zoom, main
- [x] Image : redimensionner (Lanczos), taille de la zone de travail, recadrer sur la sélection, rotation 90/180/libre, miroirs
- [x] Réglages : luminosité/contraste, teinte/saturation/luminosité, gamma, niveaux auto, balance des couleurs, seuil, postérisation, inverser, niveaux de gris, sépia
- [x] Filtres : flou gaussien, flou de mouvement, netteté, masque flou, relief, contours, pixellisation, bruit, médiane, vignettage, peinture à l'huile, cristallisation simple
- [x] Vectoriel : formes éditables (remplissage, contour, épaisseur), texte, déplacement et redimensionnement, import SVG comme calque vectoriel, rastériser un calque
- [x] Interface entièrement nouvelle, thème sombre/clair, raccourcis clavier, zoom molette, règles de zoom
- [x] Installeur Windows (NSIS `.exe` + MSI)

### Avancement du 25 octobre 2026

La première version possède maintenant un circuit complet React → IPC Tauri → Rust :
édition bitmap et vectorielle, sélections, filtres, historique, projets et exports.
Le code Rust initial interrompu a été complété ; le frontend, les commandes natives,
le rendu SVG, les entrées/sorties et l'import PFS ont été ajoutés.

Validation initiale de phase 1 : compilation TypeScript/Vite, 17 tests Rust, et parcours automatisé dans
l'application Windows avec sauvegarde/réouverture donnant le même rendu et export
dans les six formats. Le recadrage vectoriel est couvert par un test de régression.

Les cases cochées indiquent l'implémentation du périmètre initial. Les limites
actuelles (taille des images, SVG importé comme bloc, fusion en mode Normal,
cadre du texte estimé) sont détaillées dans `README.md`. Les références de
compatibilité provenant de `../photofiltre` sont consignées dans `INTEROP.md`.

Installateurs NSIS `.exe` et MSI générés ; binaire de distribution vérifié avec son interface embarquée finale et ouverture d’un projet passé en argument. Le parcours d’édition a également été rejoué avec succès sur ce binaire de distribution.

Prochain jalon : phase 2, en commençant par
les sélections lasso/polygone/baguette et leur masque, puis les outils de retouche.

## Phase 2 — Périmètre bitmap implémenté et vérifié

- [x] Sélection lasso, polygone, baguette magique, sélection par couleur, contour/bord progressif, mémorisation de sélection
- [x] Outils : tampon de clonage, doigt (smudge), éclaircir/assombrir, aérographe, tube (nozzle), correcteur
- [x] Masques et cadres (bibliothèque **à créer**, pas de reprise des ressources PhotoFiltre), motifs, textures
- [x] Courbes interactives, niveaux manuels, remplacement de couleur, mélangeur de couches, désaturation sélective
- [x] Effets : ombre portée, biseau, contour de calque, effets de texte
- [x] Mode indexé (palette PNG conservée, GIF animé en lecture), transparence par couleur
- [x] Automatisation : traitement par lot, actions enregistrées
- [x] Formats : PSD RVB 8 bits (calques bitmap), AVIF, HEIC via codec Windows, RAW (aperçu JPEG ou codec), EXIF dans projets et exports PNG/JPEG/WebP/AVIF

Les cases désignent le périmètre livré, avec les limites explicites de `README.md` :
ce n'est pas une compatibilité universelle avec tous les documents Photoshop,
les données RAW du capteur ou toutes les variantes AVIF. Les ressources et le code
de PhotoFiltre ne sont pas distribués avec Prisme.

## Phase 3 — Vectoriel avancé

- [x] Plume (courbes de Bézier), édition des nœuds, opérations booléennes (union, différence, intersection, exclusion ; courbes approchées à 0,25 px)
- [x] Import SVG simple éditable forme par forme : courbes, formes, traits et texte convertis en surfaces
- [ ] Groupes éditables, dégradés et motifs SVG, SVG complexes
- Texte sur tracé, polices variables, styles
- [x] Export SVG optimisé (normalisation, précision), PDF vectoriel (textes et formes ; effets rendus en bitmap)
- Vectorisation d'un bitmap (tracé automatique)
- Import des tracés PhotoFiltre `.pfv` comme formes

## Phase 4 — Plugins (en cours)

- [x] **API de plugins propre à Prisme** (DLL C ABI + manifest), documentée, plus scripts (Rhai ou Lua) pour des filtres simples
- [ ] Hôte de **filtres Photoshop `.8bf`** (via pspihost ou réimplémentation)
- [ ] **Plugins PhotoFiltre `.pfl` : expérimental, faisabilité partielle**
  - Faisable : découverte et chargement 64 bits, lecture de l'en-tête (offsets connus, voir `../photofiltre/PLUGIN_API.md`), menus, icône, `Execute`, `AboutPlugin`
  - Difficile : une partie des callbacks échange des objets Delphi VCL (`TBitmap`). Un hôte non-Delphi ne peut pas les fabriquer → seuls les plugins qui passent par l'API `*DBmp` (niveau ≥ 2) seront compatibles
  - Préalable : nommer les 27 + 10 callbacks de `TPluginStruct` (snapshot Ghidra prêt dans `../photofiltre/analysis/`)
  - Isolation : exécuter chaque plugin dans un processus hôte séparé pour qu'un plantage n'emporte pas l'app

## Phase 5 — Finitions

- Performances : tuiles et calcul GPU (wgpu) pour les grandes images, aperçu temps réel des filtres
- Gestion des couleurs (profils ICC), 16 bits par canal
- Traductions (FR/EN au départ), aide intégrée, mises à jour automatiques signées

### État du 25 octobre 2026 — version 1.1

Le périmètre de phase 2 est implémenté. Les parcours natifs couvrent sélections,
retouche, ressources, réglages, effets, palette, lecture GIF, actions, lots et
formats. Un corpus indépendant de neuf PSD est comparé pixel par pixel ; un
fichier HEIC et un RAW Canon sont décodés. Le style reste volontairement minimal.
La phase 3 a commencé par les exports SVG optimisé et PDF ; les autres postes
vectoriels ci-dessus restent à réaliser. Voir `IMPLEMENTATION.md` pour les preuves.

### Jalon 1.2 — placement PhotoFiltre 11 et poursuite vectorielle

- [x] Placement fonctionnel confirmé par les formulaires locaux de PhotoFiltre 11 : deux barres, calques à gauche, couleurs/outils/options à droite, grille 3 × 6.
- [x] Fusion depuis le fond avec tous les modes ; fusion d’un préfixe de la pile sans aplatir les calques supérieurs.
- [x] Export GIF animé : images coalescées, transparence, délais et nombre de passages.
- [x] Flou local, copie d’image/sélection et collage comme nouvelle image ; raccourcis de réglages.
- [x] Plume, poignées, nœuds, scission, conversion des formes simples et quatre booléens ; projets v6.

La suite de phase 3 reste ouverte : SVG décomposé, groupes et peintures SVG,
texte sur tracé, polices variables, vectorisation bitmap et import PFV. Les étapes
de plugins et de gestion des couleurs gardent leur place ultérieure dans le plan.
Les fonctions sans implémentation sont explicitement grisées dans l’interface.

### Jalon 1.3 — SVG simple éditable

- [x] Ouverture directe d’un SVG simple en tracés éditables, et décomposition d’un bloc déjà importé.
- [x] Courbes cubiques/quadratiques/arcs, primitives, texte en contours, groupes neutres dépliés, transformations, remplissages et traits pointillés en surfaces.
- [x] Sélection des surfaces selon leur géométrie, y compris à travers un trou ; réglage non nul/pair-impair et opacité de forme.
- [x] Décomposition annulable, persistance et refus sans altérer le document ni l’historique.

La décomposition reste limitée aux couleurs unies et groupes sans effet. Les
dégradés, motifs, images intégrées, groupes avec opacité/fusion, filtres et découpes
restent des blocs. Les écarts de rasterisation des traits convertis sont mesurés
et contrôlés. La phase 4 n’a pas été réalisée dans ce jalon.

### Développement après 1.3 — objectif : terminer le plan complet

L’API de plugins propre à Prisme (C ABI v1 + manifeste) et les scripts Rhai sont
implémentés avec un hôte dans un processus séparé, délai global et Job Windows.
Le SDK contient un en-tête C, un filtre C compilé et deux scripts originaux.
Le parcours natif couvre sélection, alpha, paramètres, annulation, actions, lots,
retour d’erreur, plantage réel de DLL, contrat corrompu et arrêt après délai.
Cela ne termine pas la phase 4 : hôtes Photoshop 8BF et PhotoFiltre PFL restent
à implémenter. Les autres exigences de phases 3 et 5 restent intégralement ouvertes.
Le suivi des exigences et des preuves est conservé dans EXECUTION.md.
