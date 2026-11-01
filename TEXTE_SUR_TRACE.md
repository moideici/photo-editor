# Texte sur tracé — développement v15

Ce jalon permet de placer un texte éditable sur un guide vectoriel du même
calque. Les anciens installateurs 1.4 et l’aperçu portable v11 restent figés.

## Utilisation

Créer un texte et un tracé, une ligne, une ellipse ou un rectangle sans arrondi
dans le même calque vectoriel. Sélectionner le texte, puis ouvrir « Texte sur
tracé ». Choisir son chemin et, pour un tracé composé, le contour à utiliser.
L’aperçu est calculé par le moteur natif en brouillon. Le dessin et son historique
restent inchangés avant « Appliquer le texte sur tracé ».

Le départ accepte des pixels du chemin ou un pourcentage. L’alignement se place
au début, au centre ou à la fin de ce départ ; l’écart déplace la ligne de base.
Inverser le sens change le sens de parcours du chemin. Les lettres dépassant
les extrémités du chemin peuvent ne pas apparaître, comme dans le rendu SVG ;
leur contenu reste présent dans le champ de texte et dans le projet.

Le contenu, la police, le corps, le gras, l’italique, les peintures et l’opacité
restent éditables. Les modifications du guide mettent à jour le placement des
lettres, y compris lorsque texte et guide sont dans des groupes différents.
La position du texte lié se règle par le guide, le départ et l’écart. Ses champs
de géométrie sont désactivés ; pour le transformer indépendamment, détacher
d’abord le chemin. Le guide reste un objet distinct : son fond et son trait
peuvent être retirés pour ne laisser apparaître que les lettres.

« Détacher le chemin en gardant la courbe » prépare une copie indépendante du
guide. Appliquer garde le rendu et permet ensuite de déplacer, redimensionner
ou tourner le texte sans dépendre de ce guide. « Revenir au texte droit » garde
le contenu éditable et retire le placement sur courbe. Les deux opérations sont
annulables. La suppression ou le remplacement incompatible d’un guide encore
utilisé est refusé sans modifier le document ni son historique.

« Convertir le texte en contours » est une commande distincte : les lettres
deviennent des groupes de tracés à nœuds éditables. L’édition du contenu et de
la police n’est alors plus disponible ; Annuler restaure le texte et son lien.
Cette commande est également disponible pour le texte droit.

## Projets et exports

Le projet écrit v15 et relit les versions 1 à 15. Il conserve le texte, les
réglages, une géométrie de guide et le lien vers l’identité de sa forme source.
Duplication et import réattribuent les identités ensemble. Une vraie archive
v14 du jalon feImage est migrée et comparée au rendu exact.

Le SVG courant conserve un élément textPath et un guide local autonome.
Le lien avec une autre forme du projet devient une définition de chemin propre
à cet export. Un autre éditeur peut donc éditer le texte et ce guide exporté,
mais ne reçoit pas le lien dynamique propre au modèle de Prisme. Le SVG optimisé
est également vérifié. Dans Prisme, « Décomposer le SVG en tracés » transforme
explicitement son texte en contours, y compris un texte sur tracé. Pour garder
le texte et ses liens éditables dans Prisme, conserver le projet .prisme.

Le PDF convertit les lettres courbes en contours vectoriels avec le moteur
de l’éditeur, dans une copie d’export. Le projet conserve son texte éditable.
Le PDF de référence ne contient aucune image XObject : guide et lettres sont
des géométries vectorielles. Les filtres éventuellement présents autour du texte
gardent leurs règles d’échantillonnage documentées séparément.

## Limites et précision

- Une ligne et 10 000 octets de texte maximum par texte sur tracé ; un contour
  de deux à 20 000 nœuds. Les limites de formes et de groupes restent applicables.
  Le départ est borné à ±1 000 % ou ±1 000 000 pixels et l’écart à ±4 096.
- Les liens restent dans le même calque. Pour placer du texte dans une ressource
  de filtre ou un masque, détacher son chemin au préalable. La création de texte
  lié n’est pas enregistrable dans une action fondée sur des identités de formes
  non stables ; la création d’un texte détaché reste dans le contrat existant.
- Polices variables, styles de texte réutilisables, remplacement explicite d’une
  police absente et import externe en texte lié éditable restent ouverts.
  Le rendu emploie les polices disponibles sur le système.
- La conversion des quadratiques des polices en courbes cubiques éditables peut
  changer le lissage aux arêtes. Dans le test du cœur à 28 px : maximum 41,
  moyenne 0,00936 et 106 pixels au-delà de huit niveaux sur 119 600. Le critère
  propre à cette conversion est maximum 48, moyenne inférieure à 0,02 et surface
  affectée au plus 0,1 %. Les critères du texte éditable restent distincts.
- Dans la référence Windows à 34 px : conversion en contours maximum 52,
  moyenne 0,01237 et 339 pixels au-delà de huit niveaux sur 268 800, soit 0,1261 %.
  Ses limites sont 64, 0,03 et 0,15 %. La décomposition supplémentaire du SVG
  observe maximum 52, moyenne 0,02558 et 591 pixels, soit 0,2199 %, sous des
  limites 64, 0,03 et 0,25 %. Ces écarts sont publiés et ne constituent pas une
  identité stricte des pixels. Aucune lettre ne doit disparaître à la conversion.
- Les transformations propres au texte sont conservées lors du groupement
  et de la dissociation, y compris après redimensionnement et rotation. Les
  groupes simples ont un écart nul dans les références finales. La dissociation
  du groupe affine a un écart nul dans le test du cœur ; dans la scène Windows,
  maximum 13, moyenne 0,00006417 et trois pixels au-delà de huit, sous des limites
  24, 0,01 et 0,05 % de surface. Ces faibles écarts restent des arrondis de rendu.

## Vérification du jalon

Neuf nouveaux tests du cœur couvrent références SVG indépendantes, départs et
alignements, inversion, styles, guides fermés/composés, édition des nœuds,
groupes, duplication, détachement, refus atomiques, projet v15, contours,
export PDF et sélection des pixels visibles. Les 26 contrôles Windows ajoutent
les gestes réels, le glisser sans déplacement parasite, les aperçus retardés,
les brouillons et la migration d’archive.

Validation consolidée : 136 tests du cœur et deux corpus externes, 396 contrôles
Windows, quinze comparaisons PDF et onze inspections structurelles. La nouvelle
référence PDF a une moyenne RGB de 0,11590, un maximum d’arête de 119 et
0,08371 % des pixels au-delà de 32. Avec une bande d’arête d’un pixel, les aplats
ont un écart nul, sous les anciens critères vectoriels (maximum 3, moyenne 0,5).
Ses 236 courbes de Bézier sont présentes dans le PDF, sans image XObject.

Les rapports consolidés et leurs empreintes sont regroupés dans
`outputs/Progression-texte-sur-trace/validation.json`. Les résultats finaux sont
consignés dans EXECUTION.md et BILAN_ACTUEL.md. La distribution de ces acquis et
les autres exigences du PLAN restent ouvertes.
