# Filtres et effets SVG — développement v13

Ce jalon suit les masques v12. Les distributions 1.4 et l’aperçu portable v11
restent figés et ne contiennent pas ces nouveaux acquis.

Complément courant : le jalon v14 ajoute une dix-septième famille, feImage,
avec ses sources éditables. Voir [SVG_FEIMAGE.md](SVG_FEIMAGE.md). Les nombres
et preuves ci-dessous décrivent le jalon v13 conservé.

## Utilisation

Ouvrir un SVG, puis utiliser « Décomposer le SVG en tracés ». Choisir un groupe
enfant portant des effets dans « Forme active », puis ouvrir « Filtres et effets
SVG ». Les chaînes, leurs étapes et leurs entrées sont éditables. Chaque étape
expose ses paramètres ; les courbes des quatre canaux, sources lumineuses et
entrées d’assemblage ont leurs propres réglages. Une chaîne peut être créée sur
un groupe existant. Les filtres présents dans le contenu d’un masque sont aussi
éditables depuis le panneau du masque.

L’aperçu est calculé par le moteur natif, en brouillon. Appliquer ajoute une étape
d’historique ; annuler les réglages ne modifie pas le document. Les réponses d’un
ancien aperçu sont ignorées. Retirer une chaîne conserve les enfants du groupe.
Le PDF utilise une copie du document ; l’export ne remplace jamais ses formes.

## Périmètre réalisé

| Famille | Paramètres principaux |
|---|---|
| Flou gaussien | Rayons X/Y, entrée |
| Ombre portée | Rayons, décalage X/Y, couleur, transparence |
| Décalage | X/Y, entrée |
| Matrice couleur | Matrice 4×5, saturation, rotation de teinte, luminance |
| Courbes des canaux | Identité, table, valeurs discrètes, linéaire, gamma |
| Fusion | Deux entrées et mode |
| Composition | Opérateur et coefficients arithmétiques |
| Assemblage | Liste d’entrées ordonnées |
| Couleur unie | Couleur et transparence |
| Morphologie | Érosion/dilatation et rayons X/Y |
| Convolution | Noyau, dimensions, cible, biais, diviseur, bords, alpha |
| Déplacement | Deux entrées, canaux X/Y, amplitude |
| Turbulence | Fréquences, octaves, graine, raccords, type |
| Carrelage | Entrée et région de l’étape source |
| Éclairage diffus | Relief, intensité, couleur, source lumineuse |
| Éclairage spéculaire | Relief, intensité, concentration, couleur, source |

Les sources lumineuses distante, ponctuelle et projecteur sont disponibles.
Chaque étape garde sa région et son calcul des couleurs (sRGB/linearRGB).
Les entrées nommées doivent être déclarées avant utilisation. Les références
absentes, futures ou dupliquées sont refusées avant toute mutation.

Les unités locales/globales des SVG sont normalisées en coordonnées utilisateur.
Le repère affine du filtre est conservé : les coordonnées des tracés ne sont pas
aplaties puis inversées autour d’un filtre. L’opacité, la fusion, la découpe et le
masque restent dans le même contexte que la chaîne. Cela évite de déplacer la
grille de calcul des ombres ou du carrelage par des intermédiaires supplémentaires.
Un cadre SVG éditable conserve le repère et les limites du SVG importé. Ses
enfants restent accessibles ; sa dissociation est refusée explicitement.

La sélection sur les pixels visibles inclut les effets situés hors de la forme
source. Pour une chaîne nécessitant les pixels voisins, le calcul de sélection
ne se limite pas à une image source de 1×1 pixel. Les contrôles de péremption de
calque, de geste annulé et de réponse tardive restent en place.

## Projets et exports

Écriture du projet v13 ; lecture des versions 1 à 13. Un véritable projet v12
du jalon des masques est migré et comparé. Les groupes et effets conservent leurs
identités récursives lors de la duplication et de l’import.

Le SVG conserve la définition vectorielle des effets. Le cadre peut employer
une image SVG intégrée contenant les groupes éditables sérialisés ; il ne s’agit
pas d’un bitmap. Un autre éditeur peut présenter ce cadre comme un SVG intégré.

Le convertisseur PDF emploie un moteur plus ancien que celui de l’éditeur.
Les sous-arbres filtrés sont donc échantillonnés par le moteur natif, sur la
grille de la page à 96 pixels par pouce. Les autres objets restent vectoriels.
L’opacité et la fusion extérieures sont conservées. Le texte des blocs
normalisés devient des contours dans cette copie d’export ; le projet garde
sa structure source. Les masques de luminance
gardent le traitement de couverture introduit au jalon v12.
Les cadres sont convertis en groupes ordinaires dans la copie d’export, avec
isolation et découpe lorsque nécessaire : le convertisseur ancien supprimait
sinon les images présentes à l’intérieur d’un SVG intégré. Une conversion de
ressources non représentable est refusée explicitement, avec proposition du SVG.

## Limites qui restent ouvertes

- Au jalon v13, **feImage** et ses ressources restaient en bloc. Ce point est
  traité au jalon v14, selon le contrat SVG_FEIMAGE.md. Une décomposition
  incompatible garde toujours le bloc original entier, même en présence
  d’autres objets visibles. Les primitives inconnues et les filtres éliminés
  par la normalisation restent refusés.
- Jusqu’à huit chaînes par groupe et 64 étapes par chaîne ; jusqu’à 64 entrées
  d’assemblage. Rayons limités à 512, convolution à 15×15, turbulence à dix octaves.
  Les régions sont bornées à 16 millions de pixels, avec côtés de 16 384 maximum.
- Les limites du document et de l’historique restent en vigueur. La préparation
  PDF est limitée à 16 millions de pixels cumulés, avec un refus explicite si
  le budget est dépassé. Les effets PDF ont une résolution finie ; ils ne restent
  pas tous vectoriels et ne sont pas garantis identiques à toute échelle.
- Le contrôle PDF affine du cœur observe cinq pixels d’arête au-delà de huit
  niveaux, maximum 16, moyenne 0,0870 sur quatre canaux ; les aplats ont un maximum
  observé d’un niveau, sous un seuil de deux dans cette référence. Ce n’est pas une identité
  stricte des pixels. Les comparaisons indépendantes du PDF ont leurs propres
  seuils et leurs bandes d’arête publiés dans le script et le rapport. Le
  double échantillonnage des sous-arbres filtrés demande une bande de deux
  pixels pour classifier les aplats constants : le premier contrôle sur un
  pixel trouvait deux pixels à quatre niveaux ; au-delà de deux pixels, le
  maximum tombe à un. Les huit anciennes références gardent leurs critères.
- L’édition graphique des filtres sur le canevas et la validation d’une
  prochaine distribution restent à réaliser. Le reste du PLAN demeure ouvert.

## Vérification

Les rapports finaux, références SVG indépendantes, images de comparaison,
captures de l’interface, scripts et empreintes sont regroupés dans
`outputs/Progression-filtres-SVG/validation.json`. Voir EXECUTION.md pour les
résultats finaux et les défauts détectés pendant ce jalon.
