# Trente propositions pour Prisme

État au 26 octobre 2026 : trente propositions encore à réaliser ou à vérifier,
**pas trente fonctions annoncées comme réalisées**.
Le plan actuel reste intégralement à terminer. Cette liste sépare dix correctifs
de comportements existants, dix améliorations de fonctions existantes et dix
nouvelles fonctions. Le bilan d’exécution reste dans EXECUTION.md.

## Dix correctifs (« patchs »)

Ces points sont repérés dans le code actuel ; une reproduction ciblée devra
accompagner chaque correction avant de la déclarer résolue. Deux corrections
déjà réalisées sont sorties des propositions : restauration des NumberField
vides et signalement des images SVG externes. P04 et P06 ci-dessous désignent
leurs remplaçantes ; les anciens bilans conservent les numéros historiques.

| N° | Correctif proposé | Observation et preuve attendue |
|---|---|---|
| P01 | Sélectionner une ellipse par sa surface réelle | geometry.ts utilise encore la boîte pour les ellipses ; un clic dans un coin vide doit atteindre l’objet dessous. |
| P02 | Corriger l’étirement non uniforme des formes tournées simples | operations.rs transforme encore certaines boîtes sans composer une matrice affine ; comparer au SVG étiré indépendamment. |
| P03 | Garder chaque forme éditable après miroir | Le chemin historique des formes simples les remplace par un bloc SVG ; conserver texte, nœuds et identités après deux miroirs. |
| P04 | Restaurer l’opacité après un geste interrompu | OpacityControl ne traite pas pointercancel : vérifier perte de capture/focus, synchroniser curseur et document, et éviter un réglage affiché mais non appliqué. |
| P05 | Signaler explicitement les polices absentes | Le moteur charge les polices système sans diagnostic de substitution ; nommer la police demandée et celle retenue à l’ouverture. |
| P06 | Étendre les diagnostics SVG aux images de filtres | Le diagnostic actuel parcourt image, pas feImage ; une référence externe de filtre doit aussi être nommée, sans accès implicite au fichier ou au réseau. |
| P07 | Éviter les étapes d’historique sans changement | Shape et plusieurs propriétés committent systématiquement ; réappliquer une valeur identique ne doit pas salir le document. |
| P08 | Préserver l’alpha des couleurs dans le sélecteur | Les contrôles HTML de couleur attendent du RGB à six chiffres alors que le rendu accepte davantage de syntaxes ; tester une forme avec couleur RGBA importée. |
| P09 | Garder accessible un diagnostic non acquitté | L’erreur globale disparaît après dix secondes ; conserver un accès à son texte et au chemin concerné jusqu’à acquittement. |
| P10 | Refuser une couleur vectorielle invalide avec explication | validate_shape_fields vérifie la géométrie, mais pas toutes les chaînes fill/stroke ; éviter une substitution implicite de couleur par le moteur SVG. |

## Dix améliorations de fonctions existantes

| N° | Amélioration | Gain concret |
|---|---|---|
| A11 | Arbre des formes avec recherche et réordonnancement par glisser-déposer | Manipuler de grands groupes sans parcourir un long sélecteur ; garder l’ordre de superposition visible. |
| A12 | Champs numériques avec glissement, pas fin/grossier et annulation du réglage | Ajuster rapidement position, taille et intensité ; une seule étape d’historique pour un réglage continu. |
| A13 | Meilleur échantillonnage des gestes de peinture | Espacement régulier, interpolation de pression et stabilisation réglable pour des traits cohérents à différentes vitesses. |
| A14 | Rapport de lot exportable et reprise des seuls échecs | Conserver chemins, erreurs, durées et réglages ; relancer les fichiers échoués sans retraiter les réussites. |
| A15 | Préréglages d’export comparables | Enregistrer qualité, dimensions et métadonnées ; estimer le poids et comparer les variantes avant export. |
| A16 | Diagnostic de compatibilité des plugins | Réunir architecture, API, capacités, erreurs de chargement et dernier résultat dans une vue consultable. |
| A17 | Historique avec repères nommés et comparaison visuelle | Retrouver une étape utile et comparer deux états sans modifier le document courant. |
| A18 | Recherche de commandes personnalisable | Recherche, favoris et commandes récentes réalisés au jalon expérience ; restent les raccourcis réassignables et la résolution visible des conflits. |
| A19 | Bibliothèque de ressources avec catégories et favoris | Retrouver rapidement tubes, cadres, motifs et textures ; prévisualiser leur effet dans le contexte courant. |
| A20 | Qualité de rendu adaptative pendant les gestes | Réduire la latence du déplacement ou d’un aperçu puis recalculer la qualité complète au repos, sans dégrader le résultat enregistré. |

Certaines améliorations recoupent des exigences déjà ouvertes du plan : elles
ne sont pas des raisons de reporter ces exigences après une prétendue fin.

## Dix nouvelles fonctions

| N° | Fonction nouvelle | Périmètre proposé |
|---|---|---|
| F21 | Calques de réglage non destructifs | Courbes, niveaux et couleurs rééditables, masquables, avec mélange et opacité. |
| F22 | Masques de calque et groupes d’écrêtage rééditables | Masque lié/délié, peinture du masque, désactivation temporaire et miniature propre. |
| F23 | Objets liés à une source externe | Import lié, actualisation explicite, source manquante signalée et incorporation possible dans le projet. |
| F24 | Transformation perspective et maillage | Quatre coins, grille de déformation et poignées éditables conservées dans le projet. |
| F25 | Épreuvage CMJN et contrôle d’impression | Profil d’épreuve, alerte hors gamut et inspection des séparations ; complément du pipeline ICC à réaliser. |
| F26 | Correction d’objectif | Distorsion, aberrations latérales et vignettage, avec réglages manuels et profils identifiés. |
| F27 | Fusion d’expositions HDR | Alignement, déghosting et fusion dans une profondeur adaptée, avant rendu tonal rééditable. |
| F28 | Assemblage panoramique | Points de correspondance modifiables, projection et correction des raccords. |
| F29 | Atlas de sprites | Découpe, rangement, marge/extrusion et export d’image avec métadonnées de rectangles. |
| F30 | Mesure et annotation technique | Échelle calibrée, distances/angles/surfaces et annotations vectorielles exportables. |

Les dix nouvelles fonctions nécessitent chacune un contrat de projet, undo,
interopérabilité, limites et corpus de validation. Elles ne sont pas activées
par une simple entrée de menu. Priorité suggérée après le socle convenu : P01–P10,
A11/A12/A14/A20, puis F21/F22/F24 selon les usages constatés.
