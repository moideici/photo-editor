# Images intégrées et découpes SVG — développement du 26 octobre 2026

Ces ajouts prolongent le jalon 1.4. Les installateurs 1.4 figés ne les contiennent
pas. Le plan complet reste ouvert ; voir BILAN_ACTUEL.md et PLAN.md.

## Utilisation

Ouvrir normalement un SVG le conserve en bloc. « Ouvrir un SVG éditable… » ou
« Décomposer le SVG en tracés » produit maintenant aussi des images intégrées et
des groupes découpés. Un élément incompatible refuse toute l’opération.

Sélectionner l’image dans Forme active, puis le panneau Image intégrée : aperçu
calculé par le moteur natif, interpolation et remplacement du fichier. Le
remplacement garde position, dimensions, transformation et opacité ; Annuler
retrouve les données précédentes. Un SVG interne peut être décomposé à son tour.
PNG, JPEG, GIF et WebP gardent leurs octets ; le SVG est normalisé, et ses textes
internes deviennent des contours. GIF et WebP animés utilisent leur première image.

Sélectionner un groupe puis ouvrir Découpe vectorielle. Les géométries importées
sont accessibles par étape et par tracé/groupe. Position, taille, rotation,
règle pair-impair, ancres/tangentes et matrices se règlent dans ce panneau.
Rectangle et Ellipse créent un brouillon. Appliquer crée une étape d’historique ;
Annuler les réglages restaure les valeurs du document. Retirer révèle le contenu
sans le supprimer. Dissocier un groupe découpé reste refusé pour éviter une perte.

Les coordonnées sont locales au groupe. Les formes d’une découpe forment une
union ; les découpes liées s’intersectent. Les trous pair-impair sont conservés.
Le clic respecte les découpes des parents, même pour un enfant déjà sélectionné.
Les poignées peuvent encore éditer le contenu hors découpe ; ce contenu reste
intentionnellement présent. L’édition directe de la découpe à la souris reste à faire.

Les images externes ne sont pas recherchées. Leur identifiant et la référence
absente apparaissent dans Informations du document. Une ouverture éditable les
refuse explicitement ; le document précédent reste intact. Ce diagnostic couvre
les éléments image ; son extension aux feImage de filtres reste proposée.

## Persistance, exports et limites

Projet v11, lecture des versions 1 à 11. Les données sont intégrées au JSON du
projet, avec les structures des découpes. Le portable/les installateurs 1.4 au
format v6 ne peuvent pas lire ces nouveaux projets. Garder une copie des originaux.
Images : 12 Mio de données décodées de Base64 par objet, dimensions bornées par
le moteur (16 millions de pixels, côté maximal 16 384). Le JSON global reste
limité à 16 Mio : plusieurs images proches de la limite peuvent empêcher une
sauvegarde, qui échoue explicitement sans remplacer le fichier existant.

Découpes : huit niveaux de découpes, 1 000 géométries et 20 000 nœuds ; groupes
bornés et matrices inversibles. Les contours importés sont normalisés en cubiques.
Le SVG exporté contient les données et les clipPath. Le PDF conserve les objets
compatibles ; les restrictions de peintures et fusion additive déjà documentées
restent applicables. Un débordement d’un SVG intégré est désormais contenu dans
son cadre, y compris lors d’un placement dans un document plus grand.

Le convertisseur PDF ne transmet pas l’interpolation des images : les objets
bitmap intégrés lissés sont pré-échantillonnés à leur taille de page à 96 px/pouce
dans une copie d’export. Les données du projet restent intactes. Les modes voisin
et pixel art gardent les pixels d’origine. Budget : 16 millions de pixels à
pré-échantillonner par export ; le dépassement est refusé et propose le SVG.
L’agrandissement ultérieur du PDF révèle cette résolution finie. Les bitmap
encore enfermés dans un bloc SVG non décomposé gardent le comportement du
convertisseur ; les découpes et formes compatibles restent vectorielles.

Masques et filtres SVG éditables, images dans les carreaux de motifs, ressources
liées, conversion universelle de toutes les variantes SVG et gestion générale
des fichiers animés intégrés restent hors de ce jalon.

## Vérification

Six tests du cœur : quatre formats bitmap, alpha, SVG interne, interpolation,
rencontre/remplissage/découpe de l’image, transformations, unions/trous,
objectBoundingBox, intersections liées et imbriquées, texte converti en découpe,
débordement, édition/undo/projet v11 et refus sans mutation. Les pixels sont
comparés en couleurs prémultipliées : seuil général maximal 16/255 et moyen
0,15. Pour les contours de texte, le premier seuil maximal a échoué à 48/255
sur 51 pixels de bord ; ce cas distinct accepte 64 et moins de 100 pixels au-dessus
de 8. Les intérieurs plats restent contrôlés séparément à 3/255 maximum.

Le parcours Windows work/phase3-media.cjs vérifie l’interface, le brouillon,
remplacement et erreurs, interpolations, nœuds, retrait, sélection sous la zone
cachée, projets, anciennes versions, identités, exports et diagnostics.
La référence SVG et le SVG réexporté ont un écart nul dans la fixture native ;
le SVG interne décomposé présente un écart maximal de 1/255.
Quatre PDF sont comparés avec Poppler. La première comparaison des images a
échoué : interpolation ignorée, moyenne RGB 1,07475 et maximum 31 dans les zones
faiblement variables. Après pré-échantillonnage : moyenne 0,50512, maximum global
82 sur les bords, maximum 7 dans ces zones et 1 dans les aplats constants.
Le rapport distingue les seuils des images (8 pour les zones faiblement variables,
3 pour les aplats constants) des anciens seuils vectoriels. Aucune identité de
pixels entre le PDF et le moteur de l’éditeur n’est annoncée.
Les rapports consolidés, empreintes, captures et exemples sont copiés dans
outputs/Progression-images-et-decoupes. Ces preuves décrivent un jalon de
développement, pas la clôture de toutes les exigences de PLAN.md.
