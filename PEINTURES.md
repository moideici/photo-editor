# Dégradés vectoriels éditables

> État courant : le développement écrit v12 et lit les versions 1 à 12.
> Les jalons de cette page sont historiques ; images et découpes ont été
> complétées par les masques peints dans SVG_MASQUES.md. Les limites des
> motifs complexes et des filtres restent indiquées dans PLAN.md.


État : réalisés dans le développement après 1.4, pas dans les installateurs 1.4
déjà livrés. Les motifs simples sont maintenant décrits dans MOTIFS.md ; les
SVG complexes et la validation de distribution restent ouverts au plan.

Dans Formes, sélectionner une forme puis ouvrir Peinture et dégradés SVG.
Choisir la cible Fond ou Trait et une couleur unie, un dégradé linéaire ou radial.
Régler les extrémités ou le centre/foyer/rayons, les couleurs et opacités par point,
leur position et le prolongement : extrémités, miroir ou répétition.
Il est possible d’ajouter/retirer des points, de 2 à 64.

Le repère peut être local à la forme ou dans les coordonnées de son parent.
La conversion utilise sa vraie boîte géométrique pour conserver le rendu.
La matrice SVG affine est éditable dans Transformation du dégradé et peut être
réinitialisée. Une géométrie sans surface ne permet pas une conversion locale.
Les formes d’un groupe conservent leur repère sous les transformations du parent.

Les réglages restent en brouillon. Le panneau montre la succession des couleurs,
pas un rendu temporaire de tout le document. Appliquer la peinture modifie le
document en une étape annulable ; Annuler les réglages retrouve la peinture
actuelle. Revenir à une couleur unie retire le dégradé de la cible correspondante.

Les dégradés fonctionnent sur les primitives, tracés et texte éditable. Les blocs
SVG et groupes ne reçoivent pas directement un dégradé : sélectionner/décomposer
leurs formes ou enfants. Les traits SVG importés deviennent des surfaces éditables
en gardant leur peinture. Les unités importées sont normalisées en coordonnées
du parent avec une matrice ; les paramètres restent modifiables.

Le jalon dégradés introduit le projet v8 ; le développement courant écrit v9
avec les motifs, puis v10 avec les groupes stylés ; il lisait les versions 1 à 10.
Un ancien Prisme ne peut pas ouvrir
ces nouvelles versions. Les couleurs de secours
restent distinctes des paramètres de peinture. Les valeurs invalides, matrices
non inversibles et points hors limites sont refusés sans modifier l’historique.

Preuves : quatre tests Rust et quinze contrôles dans la vraie application Windows,
work/phase3-paints-result.json avec empreintes. La référence SVG indépendante
transformée et transparente est décomposée au rendu identique ; les paramètres
importés sont réellement édités et annulés. La sauvegarde/réouverture et le SVG
exporté/réimporté gardent le rendu exact. Le PDF est maintenant comparé au rendu
natif avec Poppler sur papier blanc à 96 dpi : écart moyen 0,35784/255 et maximum
3/255 sur la fixture radial/fond et linéaire/trait. Un défaut de facteur DPI qui
décalait les peintures a été corrigé sans rastériser les formes.

Le PDF refuse explicitement les dégradés répétés/miroirs et les rayons de foyer
non nuls : le convertisseur actuel ne les restitue pas fidèlement. Ils restent
éditables dans le projet et exportables en SVG. Étendre cette couverture PDF et
vérifier les nouvelles fonctions dans la distribution restent ouverts.
