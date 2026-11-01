# Motifs vectoriels éditables

État : réalisés en développement après 1.4. Les installateurs 1.4 restent figés
et ne contiennent pas les groupes, dégradés et motifs ajoutés depuis.

Dans Formes, sélectionner une primitive, un tracé ou du texte puis ouvrir
« Peinture SVG : dégradés et motifs ». Choisir Fond ou Trait, puis Motif vectoriel.
Damier, rayures et pois transparents servent de points de départ originaux.
Le fond et le trait ont des peintures indépendantes ; couleur, dégradé et motif
s’excluent sur une même cible.

Le carreau a une origine, une largeur/hauteur, un espace de contenu et une matrice
affine. Ses dimensions mettent le contenu à l’échelle ; les bords découpent le
contenu. Les formes du carreau se règlent par position, dimensions, rotation,
opacité et couleur de fond. Ajouter des rectangles/ellipses, retirer des éléments
et changer leur ordre de superposition est possible. Les tracés importés gardent
leurs nœuds dans le projet ; leur édition graphique dans le carreau reste à faire.
Les dégradés importés dans les éléments sont conservés ; choisir une couleur
unie remplace explicitement le dégradé de cet élément.

Le repère du motif peut être local à la géométrie ou dans les coordonnées de son
parent. Le changement de repère compose la vraie boîte géométrique avec la
matrice pour conserver le rendu, y compris sur du texte. Les transformations
d’image et les booléens conservent la peinture. Une géométrie sans surface ne
permet pas de convertir vers le repère local.

L’aperçu natif montre **le contenu d’un carreau avant sa matrice et sa répétition**.
Il est calculé de manière asynchrone, limité à 128 × 128 pixels, avec une courte
temporisation ; une réponse périmée n’est pas affichée. Il ne modifie ni les
pixels du document ni l’historique. Les réglages restent en brouillon jusqu’à
Appliquer la peinture ; Annuler les réglages restaure la peinture courante.
Ce petit aperçu ne clôt pas les aperçus généraux de filtres prévus en I.3.

La décomposition de SVG simples importe les motifs, leurs unités normalisées,
matrice, origine, carreau et contenu vectoriel en paramètres modifiables.
Les transformations des enfants, viewBox, unités objectBoundingBox, couleurs,
alpha et dégradés du contenu sont couverts par les tests. Les traits deviennent
des surfaces. À l’intérieur d’un carreau, les motifs imbriqués, images, groupes
avec opacité/fusion, masques, découpes et effets restent refusés : le document
et son historique restent intacts ; le SVG peut rester un bloc rendu.

Limites : 1 à 200 formes par carreau, 20 000 nœuds au total, coordonnées finies,
matrice inversible et dimensions positives bornées à 16 384 unités. Les groupes,
blocs SVG et motifs récursifs ne sont pas admis dans un carreau éditable.
Les motifs introduisaient v9 ; le développement courant écrit **v10** et lit les
versions 1 à 10. Un ancien Prisme ne peut pas ouvrir ces nouvelles versions.
Le SVG exporté contient de vrais éléments pattern.

Preuves : trois tests Rust spécifiques et 20 contrôles dans l’application Windows,
rapport work/phase3-patterns-result.json avec empreintes. La référence SVG
transparente donne un écart prémultiplié maximal de 2/255 et moyen de 0,05356
niveau par canal. La tolérance du contrôle natif est fixée à 3 et 0,1 ; la
mise en contours et le rendu répété d’un carreau peuvent ajouter un arrondi alpha.
Les conversions de repères, projets et SVG exporté/réimporté de la fixture
restent exacts. Le PDF est aussi comparé par Poppler à 96 dpi sur papier blanc :
écart RGB moyen 2,4794/255, maximal 207 sur certaines arêtes d’un pixel entre
carreaux. 1,596 % des pixels dépassent 32 niveaux ; à distance d’une arête, le
maximum est 1 et la moyenne 0,0077. Ces écarts d’anticrénelage sont conservés dans
work/paint-pdf-result.json, avec les seuils et empreintes. La validation de
distribution reste ouverte.

Limite PDF explicite : les dégradés repeat/reflect et ceux dont le rayon du foyer
est non nul sont refusés, y compris dans un motif ou un SVG intégré. Le moteur
PDF actuel ne les représente pas fidèlement. Le message propose le SVG ; aucun
PDF trompeur n’est écrit, et le document/historique restent intacts. Cette limite
reste à lever pour achever la couverture de toutes les peintures en PDF.
