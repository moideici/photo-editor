# Prisme 1.4 — hôtes de plugins

Éditeur Windows d'images bitmap et vectorielles, écrit en Rust et React/TypeScript
avec Tauri 2. Licence MIT. L'interface est en français.

## État du chantier

Développement après le jalon 1.4 : groupes vectoriels éditables, transformations
imbriquées et projets v7 sont maintenant testés dans le binaire de développement.
Ces ajouts ne sont pas dans les installateurs 1.4 déjà livrés. Voir
[GROUPES.md](GROUPES.md) pour leur utilisation, leurs limites et leurs preuves.
L’import des groupes SVG conserve maintenant opacité, fusion, isolation et enfants
modifiables ; 17 modes de fusion sont disponibles. Voir
[SVG_GROUPES_STYLES.md](SVG_GROUPES_STYLES.md).
Les dégradés vectoriels linéaires/radiaux, fond et trait, sont aussi éditables
dans le développement courant ; import SVG et preuves dans
[PEINTURES.md](PEINTURES.md). Les motifs simples ont un carreau et un contenu
vectoriel éditables, avec aperçu natif asynchrone avant application :
[MOTIFS.md](MOTIFS.md). Images intégrées et découpes sont aussi éditables ; voir
[SVG_IMAGES_DECOUPES.md](SVG_IMAGES_DECOUPES.md). Les masques peints ont leur panneau
et leur aperçu : [SVG_MASQUES.md](SVG_MASQUES.md). Dix-sept familles d’effets SVG
sont éditables en chaînes : [SVG_FILTRES.md](SVG_FILTRES.md), puis les sources
bitmap/vectorielles de feImage dans [SVG_FEIMAGE.md](SVG_FEIMAGE.md).
Le texte sur tracé est éditable, avec guide lié, décalage, sens, aperçu et
conversion distincte en contours : [TEXTE_SUR_TRACE.md](TEXTE_SUR_TRACE.md).
Les polices variables proposent leurs axes réels et un aperçu avant application ;
les styles de texte sont réutilisables et les polices absentes ont un remplacement
explicite : [POLICES_STYLES.md](POLICES_STYLES.md).
Le projet courant est v17, lecture des versions 1 à 17.
Calque > Vectoriser le bitmap propose maintenant monochrome/couleur, seuil,
palette, simplification et arrondi avec aperçu, puis un calque de tracés éditables
en conservant le bitmap source : [VECTORISATION.md](VECTORISATION.md).
L'import des tracés PhotoFiltre PFV produit des chemins éditables : [PFV.md](PFV.md).
Les quatre lots d'expérience III.1 à III.4 sont réalisés : miniatures réelles,
calques multiples et verrous, recherche Ctrl K et aperçus de réglages, onglets et
récupération périodique. Utilisation et limites dans [EXPERIENCE.md](EXPERIENCE.md).
Le nouvel aperçu autonome du 27 octobre contient ce jalon v17 ; celui du
26 octobre conserve son contenu historique v11. Les installateurs 1.4 précédents
restent distincts, sans installation/mise à niveau finale annoncée.
[BILAN_ACTUEL.md](BILAN_ACTUEL.md) sépare ce qui
est livré, développé et restant ; [AMELIORATIONS_30.md](AMELIORATIONS_30.md)
contient les dix correctifs, dix améliorations et dix nouvelles fonctions proposés.

Le bilan du 25 octobre 2026 est dans [BILAN.md](BILAN.md). Le nouveau
[PLAN.md](PLAN.md) prévoit, dans l’ordre, la fin intégrale de l’ancien plan,
un agencement plus proche de PhotoFiltre 11, les améliorations d’expérience
(dont de vraies miniatures de calques), puis un pack original d’extensions PFL.
Le jalon 1.4 ajoute les hôtes Prisme/Rhai, 8BF et PFL expérimental. Les limites
et preuves initiales sont documentées séparément pour chaque famille. La phase 4
reste partielle : comparaison PFL dans PhotoFiltre et essai d’un plugin PFL tiers
x64 publié encore ouverts. Le rapport de livraison indique les paquets réellement
vérifiés. Les installateurs 1.3 existants restent antérieurs à ces ajouts.

Outils > Filtres Photoshop 8BF ouvre le catalogue, le choix de filtre, son dialogue,
ses préréglages et son À propos. L’application est isolée dans un processus enfant,
respecte sélection/alpha et s’enregistre dans les actions/lots. Voir [8BF.md](8BF.md)
pour le contrat x64 et les tests avec un filtre tiers original.

Outils > Plugins PhotoFiltre PFL ouvre le catalogue PFL Unicode x64. Les menus
Filtres, Image et Fichier donnent aussi accès aux types correspondants. L’hôte
séparé prend en charge les DBmp BGR24, sélection, alpha, undo et dialogues Win32
modaux ; les fonctions VCL et autres limites sont détaillées dans [PFL.md](PFL.md).
L’exemple négatif original est dans plugin-sdk/pfl/examples.

Le périmètre de phase 2 est implémenté et vérifié. La phase 3 comprend maintenant la plume Bézier,
l’édition des nœuds, les opérations booléennes, le SVG optimisé et le PDF vectoriel. L’atelier reprend le placement général de PhotoFiltre ; la correspondance
détaillée des commandes et options reste à compléter. Voir `IMPLEMENTATION.md` pour
les contrôles et `PLAN.md` pour les fonctions vectorielles restant à réaliser.

Dans le menu Sélection : lasso/polygone/baguette/couleur, combinaisons, contour,
progressivité et jusqu'à 32 sélections mémorisées par document. Ctrl ajoute, Alt
soustrait, Ctrl+Alt intersecte. Pour le polygone, Entrée, double-clic ou clic droit
ferme le tracé ; Retour arrière retire le dernier sommet.

Clonage (C) et correcteur (J) : Alt+clic définit le point source. Le clonage reste
aligné entre les traits. Doigt (S), éclaircir (O), assombrir, aérographe (A) et tube
(N) partagent la taille et l'intensité de l'outil. L'aérographe fonctionne aussi
sans déplacer la souris. Le tube accepte les tampons originaux ou une image chargée.

Dans Calque : Masque, Cadre, Motif et Texture ouvrent la bibliothèque procédurale
créée pour Prisme, avec un aperçu et l'import d'une image personnelle. L'application
agit sur les pixels du calque actif, respecte la sélection et peut être annulée.

Dans Réglages : Courbes par canal avec histogramme et points manipulables, niveaux
manuels, remplacement de couleur, mélangeur de couches et désaturation sélective.
Voir `IMPLEMENTATION.md` pour l'avancement vérifié et le travail restant.

Dans Calque > Effets du calque : ombre, contour et biseau non destructifs avec
aperçu. Ils s’appliquent aussi au texte vectoriel, qui reste éditable.

Dans Image : conversion en couleurs indexées, édition de la palette dans le panneau
latéral, passage en RVB et transparence par couleur. La conversion aplatit les
calques visibles et est annulable. Le mode indexé conserve les indices, y compris
les entrées dupliquées, à l’enregistrement PNG ; les outils restent limités aux
couleurs de la palette. Les transformations indexées utilisent le voisin le plus
proche. Les calques et effets nécessitent de repasser en RVB.

Un GIF animé dispose d’un lecteur, d’un curseur et d’une option de répétition.
« Éditer cette image » extrait une image fixe et peut être annulé. Le projet
conserve toutes les images du GIF. L’export GIF écrit toute la séquence avec ses
boucles et délais ; les autres exports produisent l’image courante.

Le panneau Actions et traitement par lot permet d’enregistrer les commandes
compatibles, de sauvegarder une action `.prisme-action`, de retirer des étapes et
de la rejouer en une seule étape annulable. Les cibles sont repérées depuis le
haut des calques et les dimensions/coordonnées sont absolues. Les imports,
l’édition d’une forme existante et le parcours d’historique sont exclus pendant
l’enregistrement. Sauvegarder et exporter ne deviennent pas des étapes d’action.
Les images personnelles d’un tube ou d’une ressource doivent rester accessibles
au chemin enregistré. Les lots créent des copies `_prisme`, ajoutent un suffixe
en cas de collision, poursuivent après erreur et peuvent être annulés entre étapes.

## Placement et tracés — version 1.2

L’atelier reprend l’emplacement fonctionnel de PhotoFiltre 11 : menus, barre des
commandes puis barre des réglages en haut, calques à gauche, couleurs puis grille
de 3 × 6 outils à droite, options sous les outils. Les outils et couleurs restent
visibles lorsque les propriétés défilent. Les icônes et le code restent propres
à Prisme. Impression, scanner, explorateur, pinceau avancé/artistique et déformation
sont grisés : ces fonctions restent à implémenter. Voir INTEROP.md.

Plume Bézier : clic pour un angle, glisser pour définir les poignées, clic sur
le premier point pour fermer, Entrée pour terminer ouvert, Maj+Entrée pour fermer,
Retour arrière pour retirer le dernier point, Échap pour abandonner.
Nœuds : sélectionner un tracé, cliquer son ancre puis déplacer l’ancre ou sa
poignée ; Alt permet une poignée indépendante. Suppr retire le nœud choisi.
Le panneau Formes permet aussi de fermer/ouvrir un contour, lisser/créer un angle,
scinder exactement un segment et choisir une seconde forme pour les booléens.
Un geste validé donne une étape annulable. Les projets v6 conservent les courbes.
Le flou local partage la taille, l’intensité et la sélection des outils de retouche.

## Démarrer en développement

Prérequis : Node.js, Rust stable (1.91 ou supérieur), outils C++ MSVC, CMake,
Perl (celui de Git for Windows est détecté) et WebView2. CMake et Perl servent à
compiler le décodeur AV1 ; ils ne sont pas nécessaires à l’application distribuée.

```powershell
cd C:\Users\G\Dev\rea\1-\prisme
npm install
npm run tauri dev
```

`npm run dev` lance uniquement l'interface dans un navigateur. L'édition et les
fichiers nécessitent le cœur natif lancé avec `npm run tauri dev`.

## Compiler et vérifier

```powershell
npm run build
npm run test:core
npm run tauri build
```

Les installateurs NSIS et MSI sont produits dans `src-tauri/target/release/bundle/`.
L'exécutable autonome est `src-tauri/target/release/prisme.exe` ; il utilise WebView2.
Les dépendances sont verrouillées dans `package-lock.json` et `src-tauri/Cargo.lock`.

## Utilisation

- Ouvrir une image ou un projet, ou créer une image avec fond blanc, sombre,
  transparent ou de la couleur active.
- Dessiner au pinceau, gommer, remplir avec tolérance, prélever une couleur et
  tracer un dégradé. Les outils bitmap et filtres agissent sur le calque actif.
- Dessiner rectangles, ellipses, lignes et textes sur des calques vectoriels.
  Un calque vectoriel est ajouté automatiquement si nécessaire. L'outil déplacement
  sélectionne une forme ; sa poignée ou le panneau Formes permet de la redimensionner.
- Le panneau Calques permet la visibilité, l'opacité, le mode de fusion et l'ordre.
  Le menu Calque propose duplication, renommage, fusion, aplatissement et rastérisation.
- Les sélections rectangle/ellipse limitent les outils bitmap et les filtres.
  Sélection > Importer une sélection PhotoFiltre charge un polygone `.pfs`.
- Image propose redimensionnement Lanczos, zone de travail, recadrage, rotations
  fixes/libres et miroirs. Les règles suivent le zoom et le déplacement de la vue.
- Enregistrer un `.prisme` conserve les calques et formes. Exporter produit une
  image aplatie ou un SVG contenant les formes et les bitmap intégrés.

Le glisser-déposer ouvre le premier fichier si aucun document n'est ouvert ; sinon,
il importe les fichiers comme calques. Une demande d'enregistrement protège les
modifications lors de la fermeture ou du remplacement du document.

### Raccourcis

| Action | Raccourci |
| --- | --- |
| Nouveau / ouvrir / enregistrer | Ctrl N / Ctrl O / Ctrl S |
| Importer comme calque / enregistrer sous | Ctrl Maj O / Ctrl Maj S |
| Annuler / rétablir | Ctrl Z / Ctrl Maj Z ou Ctrl Y |
| Tout sélectionner / désélectionner | Ctrl A / Ctrl D |
| Coller une image du presse-papiers Windows | Ctrl V |
| Adapter la vue / afficher à 100 % | Ctrl 0 / Ctrl 1 |
| Déplacement / pinceau / gomme | V / B / E |
| Sélection / remplissage / pipette / dégradé | M / G / I / D |
| Texte / rectangle / ligne | T / U / L |
| Main / zoom | H / Z |
| Main temporaire / zoom | Espace + glisser / molette |
| Permuter les couleurs / taille du pinceau | X / crochets [ et ] |
| Supprimer la forme sélectionnée | Suppr |

## Formats et limites

- Import : PNG, JPEG, WebP, BMP, GIF animé, TIFF, ICO, SVG, PSD RVB 8 bits,
  AVIF fixe, HEIC/HEIF si codec Windows disponible, aperçu RAW et `.prisme`.
  Export : PNG, JPEG/AVIF avec qualité réglable, WebP sans perte, BMP, TIFF,
  SVG standard/optimisé et PDF vectoriel.
- PSD : caches bitmap, noms Unicode, positions, visibilité, opacité, modes de
  fusion disponibles dans Prisme, masques bitmap et écrêtage appliqués aux pixels.
  Les groupes sont dépliés. Textes/objets dynamiques restent bitmap. Réglages,
  effets Photoshop et masques vectoriels non interprétés sont signalés dans
  Informations du document. PSB, CMJN et profondeurs 16/32 bits sont refusés.
- AVIF : alpha indépendant, décodage 8/10/12 bits vers RGBA 8 bits ; HDR et profils
  ICC non gérés. L’alpha prémultiplié est refusé explicitement. Les transformations
  de conteneur sont appliquées ; le recadrage `clap` est signalé sans être appliqué.
- HEIC dépend des extensions HEIF/HEVC installées. RAW extrait un aperçu JPEG
  intégré ou fourni par WIC : il ne développe pas les données du capteur. La
  résolution disponible peut être inférieure à celle de la photo originale.
- EXIF : conservé depuis les sources prises en charge par leurs décodeurs dans
  le projet et les exports PNG/JPEG/WebP/AVIF. L’orientation est appliquée une fois,
  les dimensions existantes sont actualisées, la référence à l’ancienne vignette
  est retirée. EXIF TIFF natif et métadonnées XMP/IPTC/ICC ne sont pas transférés.
  EXIF limité à 1 Mio (JPEG impose aussi la limite de taille de ses segments).
- PDF : formes et textes conservés, polices incorporées, groupes filtrés rendus
  en bitmap, page à 96 pixels/pouce. Pas de gestion d’impression ICC/PDF-X.
  Les dégradés repeat/reflect et ceux dont le rayon du foyer est non nul sont
  refusés explicitement ; la fusion additive est aussi refusée. Le SVG reste exportable. Voir PEINTURES.md et MOTIFS.md
  pour les comparaisons de rendu et les limites.
  L’optimisation SVG normalise le document et arrondit les nombres ; une précision
  réduite peut modifier les détails. Le projet reste la source éditable complète.
- RGBA 8 bits ; 16 mégapixels au maximum et 16 384 pixels par côté. Au plus 256
  calques ; chargement des pixels d'un projet limité à 512 Mio.
- GIF : 1 000 images et 256 Mio de pixels maximum, délais minimum 10 ms ; lecture
  et sauvegarde en projet, export de la séquence animée. Pas d’édition des images
  de la séquence : extraire une image fixe avant de la modifier. GIF limite les
  couleurs à 256 par image et l’alpha à 0/255 (seuil 128), arrondit les durées
  à 10 ms et accepte au plus 65 536 passages ou une boucle infinie.
- Fichier d’entrée limité à 512 Mio ; action limitée à 512 étapes et 16 Mio.
- Historique sans limite fixe d'étapes, avec budget de 256 Mio pour les instantanés
  partagés. L'état courant est conservé même s'il dépasse ce budget. L'historique
  est réinitialisé à l'ouverture. Les sélections et leur mémoire font maintenant
  partie des états annulables, et leurs masques sont partagés en mémoire.
- Un SVG ouvert normalement reste un bloc. « Ouvrir un SVG éditable… » et
  « Décomposer le SVG en tracés » acceptent les formes, courbes, textes en contours,
  couleurs unies, dégradés, motifs simples et groupes sans effet. Les traits deviennent des surfaces éditables,
  avec leur épaisseur, pointillés et extrémités ; la décomposition peut changer
  légèrement l’anticrénelage. Les textes convertis ne sont plus du texte modifiable.
  Les transformations et les règles de remplissage sont conservées ; le panneau
  Formes permet de changer la règle et l’opacité de chaque surface.
  Les dégradés et motifs simples sont éditables dans le développement courant.
  Les groupes avec opacité/fusion/isolation deviennent éditables avec leurs enfants.
  Images intégrées et découpes sont éditables dans le développement courant.
  Un débordement du cadre SVG conserve son contenu sous une découpe du cadre.
  Les masques alpha/luminance ont un contenu peint éditable ; les masques que le
  parseur élimine entièrement sont refusés explicitement. Les motifs imbriqués et
  effets feImage restent des blocs ; seize autres familles de filtres sont éditables. Les refus conservent le document
  et son historique. Une décomposition validée s’annule en une étape.
  Limites : 1 000 surfaces et 20 000 nœuds produits par SVG, 64 niveaux de groupes.
  Un miroir de formes simples sans peinture avancée produit encore un bloc SVG ;
  les groupes, images et formes avec dégradé/motif conservent leur structure affine.
  Les images SVG externes ne sont pas chargées : elles sont signalées dans les
  informations du document et empêchent une décomposition partielle.
  Taille source SVG maximale : 16 Mio.
- Le cadre du texte utilise une estimation de ses dimensions. Les polices doivent
  être installées sur la machine ; l'export SVG conserve le texte.
- Fusion vers le bas : les deux calques du fond acceptent tous les modes disponibles.
  Plus haut dans la pile, deux calques en mode Normal peuvent être fusionnés ;
  sinon « Fusionner jusqu’à ce calque » inclut le fond pour conserver le rendu et
  laisse les calques supérieurs séparés. Toutes ces fusions sont annulables.
- Une seule image est ouverte à la fois. Les groupes éditables et les dégradés/motifs
  SVG simples sont réalisés en développement après 1.4, dans leurs contrats documentés.
  Les tracés créés dans Prisme sont
  éditables nœud par nœud ; 20 000 nœuds et 1 000 contours par forme au maximum.
  Les booléens exigent deux tracés fermés avec fond et approchent les courbes à
  0,25 px ; le résultat prend le style de la première forme. Les formes simples
  convertibles sont rectangle sans arrondi, ellipse et ligne. Le texte Prisme
  garde son édition actuelle ; le texte SVG décomposé devient un contour. Les booléens et l’édition de nœuds ne sont pas
  enregistrables dans une action.
  Les plugins `.pfl`/`.8bf` ont leurs hôtes et limites dans PFL.md et 8BF.md.
  Les tracés `.pfv` et la gestion ICC restent à réaliser.
- Les exécutables et installateurs locaux ne sont pas signés.

## Architecture

`src/App.tsx` gère l'atelier et les commandes ; `src/Canvas.tsx` gère les gestes,
le zoom et les aperçus. Les commandes natives et leurs types se trouvent dans
`src-tauri/src/operations.rs`. Les autres modules séparent document/historique,
traitements bitmap, rendu SVG, fichiers et import PFS.

Le projet `.prisme` est un ZIP versionné contenant `document.json`, les images
PNG de chaque calque bitmap et les PNG en niveaux de gris des masques de sélection.
Le développement courant écrit la version 13 et lit les versions 1 à 13.
La distribution 1.4 reste au format v6 ; elle ne peut pas ouvrir les projets v7 à v13.
L’aperçu autonome Prisme-apercu-2026-10-26 est v11 et ne contient pas les masques
éditables ajoutés ensuite en développement.
Il conserve aussi les effets, palettes, images/délais GIF et EXIF.
La sauvegarde écrit un fichier temporaire dans le
même dossier avant de remplacer atomiquement le fichier cible.

Le jalon 1.4 livre l’API de plugins Prisme et Rhai : menu
Outils > Plugins Prisme, exemples disponibles, chargement de manifeste/fichier,
paramètres et exécution dans un hôte séparé. Une application réussie respecte
la sélection et s’annule ; les erreurs/plantages/délais conservent l’image.
Actions et lots acceptent ces plugins. Voir plugin-sdk/README.md pour créer un
filtre. Les DLL natives ont les droits de l’utilisateur ; le processus séparé
protège l’éditeur des plantages et ne constitue pas une sandbox de sécurité.
Les hôtes PhotoFiltre PFL expérimental et Photoshop 8BF x64 sont aussi livrés
en 1.4, avec leurs limites et preuves séparées dans PFL.md et 8BF.md.
Les installateurs 1.3 déjà produits ne contiennent pas ces développements.

Voir `PLAN.md` pour la suite et `INTEROP.md` pour les références PhotoFiltre.
Les composants tiers gardent leurs licences, regroupées dans
`THIRD_PARTY_NOTICES.txt`. Aucune ressource PhotoFiltre n’est distribuée.
