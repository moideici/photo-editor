# Prisme — bilan au 25 octobre 2026

**Repère chronologique :** ce bilan est la photographie prise avant l’exécution
du nouveau plan. Depuis sa rédaction, le raccordement 8BF a été réalisé et
32 contrôles natifs ont réussi, dont l’exécution et le rejeu de Filter Foundry
tiers. Depuis, l’hôte PFL de développement a réussi 46 contrôles natifs et une
référence officielle portée x64 ; la comparaison dans PhotoFiltre et les essais
empaquetés restent ouverts. Le corps de ce bilan conserve l’état initial.
Le suivi actuel est dans EXECUTION.md, PFL.md et PLAN.md. Le jalon 1.4 a ensuite
revalidé ces acquis dans les trois payloads portable/NSIS/MSI, 214 contrôles
par variante ; cela ne modifie pas la photographie historique ci-dessous.

L’application possède une base de retouche bitmap et vectorielle utilisable.
**Le plan précédent n’est pas terminé et la phase 4 n’est pas terminée.**
Le placement général PhotoFiltre a commencé ; les aperçus réels des calques
restent à créer.

Ce bilan repose sur le code, les documents et les rapports locaux existants.
Les tests cités n’ont pas été relancés pour cette réorganisation documentaire.
Le nouveau travail demandé ici est le bilan et le plan, sans nouvelle livraison binaire.

## Deux états à distinguer

| État | Ce qu’il contient | Niveau de preuve |
|---|---|---|
| Distribution 1.3.0 déjà produite | Bitmap, calques, sélections, retouche, formats, actions/lots, plume, booléens, SVG simple éditable, exports SVG/PDF, premier agencement PhotoFiltre | Rapport du binaire distribué : 120 contrôles natifs réussis ; 73 tests Rust et 2 corpus externes consignés |
| Développement après 1.3 | API plugins propre à Prisme et scripts Rhai, processus séparé, SDK et interface | Rapport plugins : 16 contrôles natifs réussis ; 77 tests Rust consignés, les 2 corpus non relancés dans cette commande |
| Préparation 8BF | Cœur libre ajouté comme dépendance locale | 145 tests du composant et cargo check consignés ; aucune exécution 8BF dans Prisme démontrée |
| PFL | Documentation ABI et références disponibles dans photofiltre | Aucun hôte PFL fonctionnel démontré |

Les installateurs 1.3 et le numéro de version encore présent dans le source ne
signifient pas que les derniers ajouts de développement ont été empaquetés.
Il faudra une nouvelle construction et un contrôle des fichiers réellement livrés.
Les nombres de tests de niveaux différents ne s’additionnent pas en un taux d’achèvement.

## Ce qui est déjà acquis

**Phases 1 et 2, dans leur périmètre documenté.** Ouverture/export et projets
.prisme, calques bitmap/vectoriels, historique, sélections et import PFS,
outils de peinture et retouche, transformations, réglages, effets de calque,
ressources procédurales originales, palette indexée, lecture/export GIF animé,
actions et lots. Les formats supplémentaires incluent PSD RVB 8 bits, AVIF,
HEIC selon codec Windows et aperçu RAW ; ce n’est pas un développement RAW complet.
Les métadonnées EXIF sont conservées dans les chemins documentés.

**Phase 3 partielle.** Plume Bézier, poignées et nœuds, division de segment,
conversion de formes, quatre opérations booléennes et exports SVG/PDF.
Un SVG simple peut être ouvert ou décomposé en formes éditables. Les couleurs
unies et traits sont pris en charge ; le texte importé de cette manière devient
des contours. Les groupes neutres sont aplatis ; ce ne sont pas des groupes
éditables avec leur propre opacité.

**Phase 4 partielle, en développement.** Un filtre DLL utilisant l’ABI C de Prisme
et les scripts Rhai peuvent recevoir des paramètres et modifier le calque via
un processus séparé. Les essais couvrent notamment sélection, alpha, undo,
actions, lots, erreur, crash, délai et contrat corrompu. Un échec conserve les
pixels et l’historique. Cette API propre n’est ni l’ABI Photoshop ni l’ABI PFL.

**Premier agencement PhotoFiltre.** Deux barres supérieures, calques à gauche,
couleurs et outils à droite, grille de trois colonnes et six rangées, options
sous les outils. Le relevé vient des formulaires locaux TMAINFORM/TOUTILSFORM ;
aucune comparaison visuelle directe avec une fenêtre PhotoFiltre pilotable
n’a encore confirmé l’ensemble des positions.

## Ce qui manque réellement

| Domaine | Reste à faire |
|---|---|
| Vectoriel avancé | Groupes éditables, dégradés/motifs, SVG complexes, texte sur tracé, polices variables/styles, vectorisation bitmap et import PFV |
| Photoshop 8BF | Raccorder découverte/exécution/dialogues/paramètres au processus hôte et à l’interface ; tester un vrai filtre indépendant |
| PhotoFiltre PFL | Vérifier les callbacks et structures, implémenter découverte/exécution/DBmp et compatibilité expérimentale x64, puis tester de vrais plugins |
| Précision et performances | Pipeline ICC et 16 bits, traitement par tuiles, GPU wgpu réellement utilisé et aperçu temps réel généralisé |
| Finition de l’ancien plan | FR/EN complet, aide intégrée, mises à jour signées, nouvelle distribution et régression complète |
| Ressemblance détaillée | Ordre et position des sous-options, dialogues, densité des contrôles, préférences d’atelier et comparaison par écran |
| Expérience | Vraies miniatures, organisation plus rapide des calques, réglages pratiques, récupération et travail entre documents |

Les boutons impression, scanner, explorateur, pinceau avancé, pinceau artistique
et déformation sont encore grisés. Leur emplacement existe ; leur fonction ne
doit pas être comptée comme implémentée.

Dans la liste des calques, le code affiche actuellement un damier et une icône
carrée ou de plume. **Ce ne sont pas des miniatures du contenu de chaque calque.**
Le nouveau plan place cette amélioration en tête de sa partie expérience.

Le cœur reste actuellement en 8 bits, sans gestion ICC complète, avec une limite
de 16 777 216 pixels. Le SVG complexe peut rester un bloc ; les limites de
précision, de mémoire, de formats et de conversion restent documentées dans README.md.

## Réponse précise sur la phase 4

La réponse est **partiellement faite** :

- API DLL Prisme + scripts Rhai : implémentés et vérifiés en développement.
- 8BF : composant préparé et testé séparément, intégration applicative encore ouverte.
- PFL : références préparées, hôte et preuve d’exécution encore ouverts.
- Distribution contenant ces ajouts : à reconstruire et à vérifier.

Un fichier .pfl ne sera pas une DLL Prisme renommée. Pour valider cette branche,
il devra employer les exports, structures et callbacks de PhotoFiltre.
Les échanges historiques d’objets Delphi VCL limitent la compatibilité envisageable ;
le plan cible d’abord le sous-ensemble DBmp Unicode x64 réellement démontré.

## Nouveau plan retenu

1. **Finir l’ancien plan sans supprimer ses exigences** : plugins 8BF/PFL,
   vectoriel avancé, couleur/précision, performances, aperçus, langues/aide,
   mises à jour signées et livraison vérifiée.
2. **Rapprocher fortement l’agencement de PhotoFiltre 11** : menus, barres,
   calques, couleurs, outils, sous-options et dialogues ; compléter les
   emplacements encore inactifs et vérifier différentes tailles/DPI.
3. **Améliorer le confort d’utilisation** : miniatures réelles en priorité,
   manipulation des calques, raccourcis et préréglages, avant/après, récupération
   et plusieurs documents.
4. **Ajouter un pack original de cinq véritables extensions PFL** : grain argentique,
   palette pixel-art, aberration chromatique, texture raccordable et carte normale.
   Ce pack enrichira l’application et confirmera le fonctionnement de l’hôte PFL,
   avec comparaison dans PhotoFiltre et test distinct d’un plugin tiers.

Le périmètre de base est déjà important. L’ajout PFL est retenu pour son utilité
et comme épreuve concrète de la phase 4, avec des filtres distincts du négatif et
de la luminosité déjà utilisés pour tester l’API propre à Prisme.

Voir [PLAN.md](PLAN.md) pour les tâches, dépendances et critères d’acceptation.
L’ancien plan est conservé intégralement dans
[PLAN_PRECEDENT_2026-10-25.md](PLAN_PRECEDENT_2026-10-25.md).
La prochaine tâche de code est le raccordement 8BF ; aucune des cases nouvelles
n’est annoncée comme réalisée par ce bilan.

## Traces consultées

- PLAN_PRECEDENT_2026-10-25.md, EXECUTION.md, IMPLEMENTATION.md, README.md et INTEROP.md.
- package.json : version 1.3.0.
- work/release-suite-result.json : binaire 1.3.0, 120 contrôles, passed=true.
- work/phase4-plugins-result.json : 16 contrôles, passed=true, errors=[].
- src-tauri/src/plugin_runtime.rs, src-tauri/src/plugins.rs, plugin-sdk/.
- vendor/eightbf-core/UPSTREAM.md et work/eightbf-core-tests.jsonl.
- src/App.tsx : liste des calques et icônes ; src/CommandBars.tsx : commandes inactives.
- work/photofiltre-layout/ et ../photofiltre/PLUGIN_API.md.

Les rapports détaillés restent dans le projet de développement. Ce bilan et le
plan sont copiés ensemble dans le dossier de livrables de cette conversation.

