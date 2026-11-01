# Suivi d’exécution — Prisme 1.3

Objectif actif : terminer PLAN.md dans son intégralité. Les jalons ci-dessous conservent leur portée historique ; le suivi courant est dans EXECUTION.md.
Interface minimale ; style définitif à définir avec l’utilisateur. Références PhotoFiltre : INTEROP.md.

## Audit de phase 1

- [x] Revalider les fonctions bitmap, vectorielles, fichiers et historique après les extensions du modèle.
- [x] Retirer la présentation de démonstration.
- [x] Corriger recadrage/rastérisation hors canevas, miroirs éditables, géométrie de sélection et zoom ancré.
- [x] Convertir les saisies numériques des filtres avant l’IPC.
- [x] Normaliser les canaux flottants du redimensionnement Lanczos : les couleurs ne sont plus écrêtées à 1.
- [x] Contrôler le budget de rechargement avant de remplacer un projet.
- [x] Reconstruire et vérifier les paquets Windows finaux 1.3.0.

## Phase 2

- [x] Sélections : lasso, polygone, baguette, couleur, masque, combinaisons, contour, progressivité, mémoire et historique.
- [x] Retouche : clonage aligné, correcteur, doigt, éclaircir/assombrir, aérographe stationnaire, tube original/personnel.
- [x] Bibliothèque originale : masques, cadres, motifs, textures, aperçu et import personnel.
- [x] Courbes interactives PCHIP et histogramme, niveaux, remplacement, mélangeur et désaturation sélective.
- [x] Ombre, contour et biseau non destructifs, aperçu, texte toujours éditable, projet et export SVG.
- [x] Palette réelle RGBA/indices, alpha, transparence par couleur, tramage, PNG PLTE/tRNS sans perte des indices.
- [x] GIF : composition/disposition, délais, répétition, lecture/pause, image courante, projet animé, extraction annulable.
- [x] Actions : commandes compatibles enregistrées, cibles par rang de calque, fichiers JSON, retrait d’étapes, rejeu atomique annulable.
- [x] Lots : sorties sans remplacement, poursuite après erreur, progression, annulation entre étapes/fichiers, document ouvert conservé.
- [x] PSD RVB 8 bits : calques, positions, noms Unicode, visibilité/opacité, fusions courantes, masques bitmap et écrêtage appliqués aux pixels, Raw/RLE/ZIP/prédiction.
- [x] AVIF : lecture/écriture avec alpha indépendant, sans codec système.
- [x] HEIC/HEIF : WIC si codecs installés, erreur explicite sinon.
- [x] RAW : plus grand aperçu JPEG exploitable, repli sur aperçu WIC ; aucun développement capteur.
- [x] EXIF : projet et PNG/JPEG/WebP/AVIF, orientation appliquée une fois, dimensions normalisées, ancienne vignette déliée.

Les limites précises sont dans README.md. PSD ne signifie pas un aller-retour Photoshop :
textes/objets dynamiques sont des caches bitmap ; les réglages, effets et masques vectoriels
non pris en charge sont signalés dans Informations du document. Les groupes sont dépliés.

## Phase 3

Ces cases conservent le suivi du jalon 1.3. Le développement courant ajoute
les groupes, peintures, ressources SVG, texte sur tracé et polices/styles décrits
dans EXECUTION.md, puis la vectorisation bitmap de VECTORISATION.md. Le projet
reste v16 ; l’import PFV et la validation de distribution restent ouverts.

- [x] Export SVG optimisé et précision réglable 2–8 décimales, texte conservé.
- [x] PDF vectoriel, police incorporée et texte sélectionnable, 96 px/pouce ; groupes à effets rendus en bitmap.
- [x] Plume, nœuds et opérations booléennes (jalon 1.2).
- [x] SVG simple éditable forme par forme (jalon 1.3).
- [ ] Groupes, dégradés, motifs et SVG complexes éditables.
- [ ] Texte sur tracé, polices variables et styles.
- [ ] Vectorisation bitmap et import PFV.

## Validation

npm run build et npm run test:core : 73 tests automatiques passent. Deux tests de corpus
indépendants, lancés séparément avec --ignored (PSD et appareils photo), passent aussi.
Couverture : rollback, budgets, alpha, masques, indices dupliqués, géométrie, effets et persistance.

Parcours exécutés dans le binaire Tauri Windows :

- work/native-smoke.cjs : création, pinceau, formes, texte, déplacement, historique, sélection/filtres, projet identique, six exports, PFS, rotation.
- work/phase2-selections.cjs, phase2-retouch.cjs, phase2-adjustments.cjs et phase2-assets.cjs.
- work/phase2-effects.cjs : aperçu sans modification, trois effets, texte éditable, sauvegarde/SVG identiques, rotation, rastérisation sans double effet.
- work/phase2-indexed.cjs : palette/alpha, vrai PNG indexé, RVB, GIF, projet animé et extraction annulable.
- work/phase2-automation.cjs : enregistrement, fichier d’action, nouveaux IDs, rejeu en une étape, lots, collisions, erreurs et annulation.
- work/phase2-formats.cjs : PSD avec calques, AVIF, aperçu RAW et HEIC.
- work/phase3-exports.cjs : SVG optimisé et contrôles d’export PDF/SVG.

Rapports *-result.json et captures dans work/. Aucun échec JavaScript relevé.
Neuf PSD de psd-tools sont comparés à son rendu indépendant (work/compare-psd.py) :
écart maximal 0 ou 1 par canal sur 255. Les fixtures synthétiques testent aussi les quatre
compressions PSD et les fichiers tronqués. HEIC testé en 1280 × 854 ; aperçu RAW Canon en
1936 × 1288. Fichiers de référence et rapports dans work/format-fixtures/.

PDF vérifié avec pypdf et rendu par Poppler : taille correcte, texte extractible, police
incorporée, formes et disposition contrôlées visuellement. Les groupes filtrés restent bitmap.

## Persistance et distribution

ZIP de projet version 6 : document.json, calques PNG, masques de sélection PNG,
images/délais GIF, indices et bloc EXIF séparé. Versions 1 à 5 toujours lisibles.
Pixels, masques, indices, animations et EXIF sont partagés dans l’historique.
Écritures atomiques pour projets/exports ; noms nouveaux réservés avec create_new pour les lots.

Les anciens paquets 1.0 ne contiennent pas ces extensions. Les paquets NSIS et MSI 1.3.0 sont
reconstruits. Tous les parcours ci-dessus passent sur le binaire de distribution, avec vérification
du hash de son interface embarquée. Les payloads des deux paquets, la documentation et les licences
ont été extraits et contrôlés sans installation système. Rapports release-smoke-result.json
et package-result.json dans work/. THIRD_PARTY_NOTICES.txt rassemble les licences des composants
et les liens vers leurs sources exactes.
Aucune ressource PhotoFiltre n’est distribuée. Exécutables non signés.

## Jalon 1.2

ToolPalette et CommandBars reproduisent l’ordre fonctionnel des contrôles de
PhotoFiltre 11 relevé dans ses formulaires, avec des icônes Lucide. Les calques
occupent un panneau gauche redimensionnable. Le panneau droit garde les couleurs
et les outils fixes ; les options et propriétés défilent au-dessous.

paths.rs stocke des contours cubiques normalisés et les rend en SVG. PathTools
gère plume, ancres et poignées ; PathPanel gère scission de De Casteljau et booléens.
i_overlay calcule les booléens sur des contours approchés à 0,25 px, bornés à
100 000 points de calcul. Le résultat garde le style de la première forme.
L’entrée composée doit utiliser pair-impair. Les limites précises sont dans README.

MergePrefix compose seulement les calques depuis le fond jusqu’à la cible.
L’export GIF réencode les images coalescées avec une palette locale, une disposition
Background, les délais arrondis et les boucles converties entre passages et répétitions.
La limite mémoire des grandes images reste inchangée ; le travail par tuiles demeure
nécessaire avant de la relever proprement.

milestone12.rs vérifie les appartenances aux quatre résultats booléens, un trou,
un résultat vide, une entrée tournée, les sauvegardes de courbes, SVG/PDF et undo,
les huit modes de fusion et les GIF transparents à 1/3/65 536 passages ou infinis.
Le flou local vérifie les pixels hors empreinte et l’alpha. milestone12-ui.cjs
vérifie la géométrie des panneaux, les 18 emplacements, le dessin à la souris,
les ancres/poignées, la scission, la conversion et l’union via les contrôles visibles.
Les dix parcours précédents sont rejoués ; les deux corpus externes sont conservés.

## Jalon 1.3

svg_edit.rs décompose explicitement un SVG simple via l’arbre usvg. Les primitives,
arcs et quadratiques deviennent des contours cubiques ; les traits sont étendus
avant leur transformation afin de conserver les pointillés, jonctions et extrémités.
Leur ordre de peinture est conservé. L’ajustement du cadre et la rotation du bloc
sont appliqués aux coordonnées finales. Les groupes neutres sont dépliés et le
texte est converti en contours, sans prétendre conserver son édition typographique.

OpenSvgEditable charge un nouveau document seulement après décomposition complète.
DecomposeSvg remplace le bloc seulement après validation complète puis crée une
étape d’historique. Les refus n’installent pas de résultat partiel. Les opacités
de groupes ne sont pas distribuées sur leurs enfants : ces groupes restent refusés.
Le format de projet reste v6 : les résultats utilisent les tracés existants.

Les cinq tests svg_edit couvrent primitives, arcs, quadratiques, règles de remplissage,
transformations imbriquées et non uniformes, pointillés, texte, fermeture suivie
d’un segment, sauvegarde et annulation, refus atomiques et ouverture directe.
phase3-svg-edit.cjs couvre quatorze parcours/checks via les commandes visibles et
la souris. Le rendu des traits convertis diffère légèrement par anticrénelage :
la fixture couvre ce point avec erreur moyenne prémultipliée < 0,1 / 255,
maximum <= 80 et moins de 0,2 % de pixels avec un écart > 16 / 255.
La vérification visuelle accompagne ces mesures. La sélection Path2D respecte
les trous et les contours, plutôt que la seule boîte englobante.

Phase 4 : aucune API de plugins, aucun moteur de scripts et aucun hôte PFL/8BF
n’est implémenté. Les données du dossier photofiltre restent des références pour
une étape ultérieure, sans chargement de ses DLL.

## Développement après 1.3 — plugins propres et scripts

API C ABI v1 et manifestes documentés dans plugin-sdk/README.md. Le code des DLL
et scripts s’exécute dans un autre processus (mode --prisme-plugin-host-v1), avec
Job Windows de 512 Mio/processus et délai contrôlé par le parent. Les résultats
sont validés avant modification du document. Le catalogue ne charge aucune DLL.

Le panneau Plugins Prisme, accessible depuis Outils, fournit deux exemples Rhai,
le chargement de manifeste par dialogue ou chemin, le scan de dossier et les
paramètres numériques. Le SDK C inclut un filtre négatif et un en-tête compilés
avec MSVC x64. Le chargement désactive l’application pendant le changement de filtre.
Une exécution réussie respecte la sélection, conserve l’alpha si le filtre le
conserve et se rejoue via les actions/lots avec une étape d’annulation.

77 tests Rust passent (deux corpus ignorés par cette commande). Le parcours
work/phase4-plugins.cjs a validé 16 contrôles réels, dont un crash par accès nul
dans une DLL C, une erreur de contrat et un délai dépassé, avec document intact.
Les parcours native-smoke, milestone12-ui et phase3-svg-edit repassent aussi.
Le cœur indépendant eightbf-host est préparé dans vendor/eightbf-core et ses
145 tests passent, mais son intégration est encore à faire. Aucun hôte PFL n’est
encore implémenté. Les phases 3 et 5 restent ouvertes. Ces développements ne sont
pas présents dans les installateurs 1.3 existants : reconstruction finale prévue.

## Après le nouveau plan — hôte 8BF en développement

L’hôte 8BF est raccordé à Prisme. Voir 8BF.md et EXECUTION.md pour son contrat,
les 32 contrôles natifs et le filtre tiers original Filter Foundry utilisé comme
référence. Le rejeu de descripteurs sans sélecteur Parameters corrige un cas
réel où un filtre forçait son dialogue ou perdait son initialisation privée.
79 tests Prisme et 145 tests du cœur 8BF passent ; les régressions Prisme/Rhai,
natif, jalon 1.2 et SVG passent. Les installateurs 1.3 ne sont pas reconstruits.
PFL et les autres parties du plan restent ouverts.

## Lots III.1 à III.4 et import PFV — 27 octobre 2026

Quatre lots réalisés ; contrats et limites dans EXPERIENCE.md et PFV.md.
LayerList calcule les miniatures isolées visibles, gère sélection multiple,
renommage/ordre/isolation et verrous enregistrés en v17. Les gardes du moteur
protègent aussi les outils et retours de plugins. CommandSearch et RasterPreview
ajoutent recherche/préférences et brouillons avant/après. Workspace conserve
les éditeurs/historiques et les vues des onglets, récents et récupérations atomiques.
Le stockage QA est séparé des données utilisateur. Sources et preuves dans
outputs/Progression-experience-PFV, nouvel aperçu autonome distinct des anciennes
versions. 162 tests du cœur, deux corpus externes, 477 contrôles natifs plus cinq
avec redémarrage, 19 rendus PDF et 15 inspections structurelles sont consignés.
