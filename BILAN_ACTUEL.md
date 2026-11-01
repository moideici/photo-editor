# Prisme — bilan courant du 27 octobre 2026

**L’application n’est pas encore terminée au sens du plan complet.** Ce document
sépare la distribution vérifiée, le développement en cours et le travail restant.
Il complète BILAN.md, qui conserve la photographie antérieure au nouveau plan.

## Dernier jalon : quatre lots d'expérience terminés

Le 27 octobre, les lots **III.1, III.2, III.3 et III.4** sont réalisés :
miniatures réelles et cache visible, recherche/manipulation multiple des calques
et verrous moteur, commandes pratiques et aperçus de réglages, onglets indépendants,
fichiers récents et récupération automatique après interruption.
L'import **PFV** est ajouté en complément : dix vrais fichiers de référence sont
comparés à un lecteur SVG indépendant, avec géométrie et pixels concordants.

Le projet écrit est désormais **v17**, lecture 1 à 17, avec migration vérifiée
d'une archive v16 figée. La récupération est déclenchée à 60 s et les deux documents
d'un arrêt de processus sont restaurés à pixels et verrous identiques. Les détails,
limites et preuves figurent dans [EXPERIENCE.md](EXPERIENCE.md) et [PFV.md](PFV.md).
**Il reste 10 grands lots ouverts sur les 14 du nouveau plan.** Le nouvel aperçu
portable du 27 octobre est autonome ; les installateurs antérieurs restent distincts.
La partie III n'implique ni calques de réglage ni masques bitmap rééditables,
ni traitement GPU/ICC, ni compatibilité universelle des plugins.

## Réalisé et livré : jalon 1.4

- Éditeur bitmap/vectoriel, calques, historique et projets .prisme ; outils de
  peinture, retouche, transformations, sélections progressives et sélections mémorisées.
- Niveaux/courbes/couleurs, effets de calque, ressources originales et palette indexée.
- GIF animé, actions et lots dans leurs contrats documentés.
- Plume, nœuds, booléens, décomposition de SVG simples et exports SVG/PDF.
- Formats dont PSD RVB 8 bits avec calques, AVIF, HEIC selon codec Windows et
  aperçu JPEG de RAW. L’aperçu RAW n’est pas un développement du capteur.
- Plugins Prisme/Rhai et 8BF x64 ; PFL Unicode x64 expérimental dans un
  sous-ensemble documenté. Isolation, erreurs/délai/crash, sélection/alpha et undo vérifiés.
- Premier agencement proche de PhotoFiltre : deux barres en haut, calques à
  gauche, couleurs/outils/options à droite. Le relevé détaillé reste à compléter.

Preuves 1.4 : 214 contrôles sur le portable et sur chacun des payloads extraits
NSIS/MSI, avec ressources embarquées contrôlées et rapports liés aux empreintes.
L’installation et la mise à niveau complètes ne sont pas prouvées par ces essais.
Les paquets restent disponibles dans outputs/Prisme-1.4.0 ; leurs fichiers sont figés.

## Réalisé dans le développement après 1.4

Groupes vectoriels imbriqués, sélection/navigation des enfants, transformations
affines, édition à la souris et des nœuds, styles, sauvegarde v7 et migration des
anciens projets. Six tests spécifiques et 15 contrôles natifs sont consignés.
Les groupes translucides ne sont pas dissociés silencieusement : la commande
refuse le cas qui changerait les recouvrements. Voir GROUPES.md.

Import SVG de groupes avec opacité, fusion et isolation : hiérarchie et enfants
éditables, 17 modes, six tests Rust et 19 contrôles Windows. Import et duplication
réattribuent aussi les identités des descendants. SVG réexporté au rendu exact
dans la fixture ; voir SVG_GROUPES_STYLES.md. Les champs numériques vides sont
restaurés et une tabulation ne doit plus arrondir la géométrie (ancienne proposition P04).

Dégradés linéaires et radiaux éditables : fond/trait, texte, points/couleurs/alpha,
prolongement, repères et matrice ; import SVG et projet v8. Quatre tests Rust et
15 contrôles natifs passent, dont une référence SVG indépendante décomposée avec
écart de pixels nul. Les réglages restent en brouillon jusqu’à Appliquer.
Voir PEINTURES.md.

Motifs vectoriels simples sur fond/trait : carreau, contenu et superposition,
repères/matrice, modèles originaux, aperçu natif asynchrone en brouillon, import
SVG et persistance v9. Trois tests Rust spécifiques et 20 contrôles natifs.
Les cas complexes restent refusés sans perte du document. Voir MOTIFS.md.
Le décalage des peintures PDF à 96 dpi est corrigé et comparé avec Poppler.
Les dégradés répétés/miroirs et rayons de foyer non nuls restent refusés en PDF,
avec un message proposant le SVG, plutôt que produire un rendu incorrect.
Les installateurs 1.4 restent antérieurs à ces ajouts.

Images SVG intégrées PNG/JPEG/GIF/WebP/SVG : données autonomes, aperçu natif,
remplacement conservant la géométrie, six interpolations et décomposition du SVG
interne. Découpes vectorielles : unions, trous, intersections, repères, nœuds,
modèles et retrait non destructif, avec réglages en brouillon. Le projet v11
conserve ces structures et relit un authentique projet v10 au rendu identique.
Six tests du cœur couvrent aussi les formats, transformations et refus atomiques.
Le diagnostic d’une image externe absente et la sélection d’un enfant caché sont
corrigés. Voir SVG_IMAGES_DECOUPES.md pour les preuves et limites.
Ce jalon totalisait 107 réussites ; les deux corpus externes PSD et HEIC/RAW ont
également été rejoués avec succès. Le succès HEIC accepte aussi le diagnostic
explicite d’un codec indisponible ; il ne prouve pas un décodage universel.
Dans le corpus actuel, le HEIC est effectivement décodé en 1 280 × 854 et le
JPEG du RAW en 1 936 × 1 288 ; les données du capteur restent non développées.
La validation de ce jalon passait : **300 contrôles Windows**,
quatre comparaisons PDF avec seuils et écarts publiés, aucune erreur JavaScript.
Les trois premières références PDF gardent leurs seuils ; la nouvelle référence
bitmap distingue aplats constants et transitions lissées. Le PDF pré-échantillonne
les images intégrées lissées dans une copie d’export à 96 px/pouce, avec limites
explicites. Les données du projet et les formes vectorielles restent conservées.

### Ajout suivant : masques SVG peints, projet v12

Masques alpha/luminance, intersections et groupes avec contenus peints éditables :
géométrie, couleur, dégradé, image, ordre, nœuds et matrices. Aperçu natif en
brouillon avec résultats périmés ignorés, retrait/annulation et sélection tenant
compte des pixels visibles. Le parseur ne peut pas éliminer silencieusement un
contenu caché : les masques concernés restent en bloc avec une explication.
Le projet v12 relit les versions 1 à 12, dont un véritable projet v11 figé.

Au jalon des masques, le cœur passait **114 tests et les deux corpus externes**. La régression
consolidée passe **327 contrôles Windows**, dont 27 ciblés sur les masques.
Import/décomposition et réimport SVG exporté ont un écart nul dans la référence
de luminance. Huit PDF sont comparés avec leurs seuils publiés ; la couverture de
luminance est préparée en alpha à 96 px/pouce dans une copie d’export. Les tracés
vectoriels du contenu restent présents dans les quatre nouvelles références PDF.
Voir SVG_MASQUES.md pour les preuves et les limites. Les masques de calque bitmap
et les autres exigences de F22 restent à réaliser.

### Ajout suivant : chaînes d’effets SVG, projet v13

Seize familles d’effets deviennent éditables avec leurs paramètres, entrées
nommées, régions, repères et sous-éléments (canaux, éclairages, assemblages).
Brouillon, aperçu natif, annulation, suppression sans perte des enfants,
sélection des pixels déplacés et migration d’un authentique projet v12 sont
vérifiés. Les filtres présents dans un masque sont également convertibles.

Le PDF échantillonne les sous-arbres filtrés avec le moteur de l’éditeur, à
96 pixels par pouce ; les objets voisins restent vectoriels. La préparation
corrige aussi la perte d’images des SVG imbriqués dans le convertisseur PDF.
Le document source reste inchangé. À ce jalon, feImage et ses ressources
restaient à faire ; ils sont réalisés au jalon suivant décrit ci-dessous.
La validation de ces acquis dans une prochaine distribution reste ouverte.
Le cœur passe **121 tests et deux corpus externes**. La validation finale
regroupe **349 contrôles Windows**, dont 22 ciblés sur les effets, et
**11 comparaisons PDF**. Sept inspections PDF vérifient la présence conjointe
de géométries vectorielles et d’images d’effets/couverture. Les seuils d’arête
et les écarts observés restent publiés. Les preuves finales sont regroupées
dans outputs/Progression-filtres-SVG.
Voir SVG_FILTRES.md pour le contrat et les limites.

### Ajout précédent : sources feImage éditables, projet v14

La dix-septième famille du catalogue accepte des images PNG/JPEG/GIF/WebP,
des SVG intégrés et des références vectorielles locales converties en copies
autonomes. Le panneau propose aperçu natif, remplacement, trois ajustements,
neuf alignements, géométrie, opacité, matrices, ordre, peintures et nœuds.
Les groupes de la source gardent leurs découpes, masques et effets. Chargement
et réglages restent en brouillon ; une réponse tardive ne remplace pas la cible
courante. L’application est annulable en une étape.

Le projet v14 relit les versions 1 à 14, dont une vraie archive v13 figée.
Les SVG sources gardent les ombres débordant de leur cadre et les entrées de
leurs éclairages. Les références manquantes, externes ou éliminées par le
parseur entraînent un refus explicite de conversion et la conservation du bloc.
L’export PDF garde le contexte du cadre lors de l’échantillonnage natif.

Validation consolidée : **127 tests du cœur et deux corpus externes**,
**370 contrôles Windows** dont 21 nouveaux, **14 comparaisons PDF** et
**10 inspections structurelles PDF**. Les références bitmap et vectorielle
ont un écart natif nul après conversion ; le SVG exporté testé se réimporte
avec un écart nul. Le PDF conserve les objets voisins vectoriels et ne
modifie pas le projet. Son rendu reste échantillonné : deux nouvelles scènes
bitmap demandent une bande de deux pixels pour classifier les transitions
douces ; six pixels du premier contrôle à un pixel atteignaient 12 niveaux.
Au-delà de cette bande, maximum 7 ; les aplats constants 3×3 restent à 2.
Les anciens seuils sont conservés et les écarts complets sont publiés.

Contrat et limites : SVG_FEIMAGE.md. Preuves figées dans
outputs/Progression-feImage-SVG. Les installateurs et anciens aperçus ne sont
pas remplacés. Le texte sur tracé est traité au jalon suivant ; le plan global
reste ouvert.

### Ajout précédent : texte sur tracé, projet v15

Le texte suit un guide du même calque : tracé Bézier, ligne, ellipse ou rectangle
sans arrondi, avec choix du contour. Départ en pixels/pourcentage, alignement,
écart et sens sont réglables avec aperçu natif en brouillon. Contenu, police,
corps, gras, italique et peintures restent éditables. L’application est annulable.
Les nœuds et déplacements du guide mettent à jour les lettres, même entre
groupes. Duplication et import réattribuent ensemble les identités et les liens.
La dissociation après redimensionnement et rotation conserve aussi le repère
et la taille des lettres ; un test spécifique empêche leur remise à l’échelle initiale.

Le détachement garde la courbe et rend le texte indépendant ; le retour au texte
droit et la conversion explicite en contours sont distincts et annulables.
Supprimer un guide encore utilisé est refusé sans changer le document ni son
historique. Le glisser sur un texte lié ne déplace ni son cadre ni le calque.
Les sauvegardes v15 relisent v1–v15 ; une vraie archive v14 est migrée au rendu exact.

Validation consolidée : **136 tests du cœur et deux corpus externes**,
**396 contrôles Windows**, dont **26 nouveaux**, **15 comparaisons PDF** et
**11 inspections structurelles PDF**. Le texte lié et les SVG exportés de la
référence Windows ont un écart de pixels nul. Les contours peuvent avoir des
écarts d’arête : maximum 52 et moyenne 0,01237 dans la conversion native, puis
0,02558 après décomposition du SVG. Les seuils propres à chaque contrôle sont
publiés dans TEXTE_SUR_TRACE.md. Le PDF de cette scène garde guide et lettres
vectoriels, sans image XObject ; son écart moyen RGB est 0,11590 et ses aplats
ont un écart nul avec une bande d’arête d’un pixel.

Preuves figées dans outputs/Progression-texte-sur-trace. La distribution reste
ouverte. L’import SVG décomposé produit explicitement des contours ; le projet
.prisme conserve le texte et ses liens éditables. La prochaine étape concerne
les polices variables et les styles, réalisés au jalon suivant ci-dessous.

### Ajout précédent : polices variables et styles de texte, projet v16

Le panneau recense les polices installées et lit leurs axes fvar réels, avec
plages, valeurs, curseurs et aperçu natif en brouillon. Axes explicites, taille
optique automatique, réinitialisation, application et annulation sont disponibles
sur texte droit et courbe. Une police absente garde son nom et affiche le repli
utilisé ; le remplacement par une police installée est explicite et annulable.
Les valeurs provenant d’un autre poste sont préservées avec diagnostic.

Les styles copient typographie et peintures sans contenu, position ni guide.
Ils s’appliquent à un texte ou aux textes du calque en une étape annulable.
Les modèles peuvent être mis à jour, supprimés, importés/exportés et restent
dans le projet et son historique. Ce sont des copies, sans liaison permanente
entre modèle et textes. Les fichiers de police ne sont pas intégrés.

Le projet v16 relit v1–v16 et migre une véritable archive v15 au rendu exact.
Le SVG conserve texte et axes. L’optimisation conserve le SVG source lorsque
son sérialiseur perdrait les variations. Le PDF prépare les lettres variables
en contours vectoriels depuis une copie. La sélection teste aussi les glyphes
qui débordent du cadre nominal ; le panneau ne déborde plus horizontalement.

Validation : **143 tests du cœur et deux corpus externes**, **419 contrôles
Windows**, dont **23 nouveaux**, **17 comparaisons PDF** et **13 inspections
structurelles PDF**. Les deux références FontTools de Segoe UI Variable et le
SVG indépendant de texte courbe ont un écart nul. Une police variable originale
réservée aux tests fournit aussi des références indépendantes du poste.
Les nouveaux PDF ont des moyennes RGB de 0,25899 et 0,12434, des maxima d’arête
de 96 et 104 ; les aplats ont un écart nul avec la bande d’arête existante.
Leurs lettres sont des chemins vectoriels sans image XObject.

Voir POLICES_STYLES.md et outputs/Progression-polices-styles pour le contrat,
les preuves et les sources v16. Les paquets existants restent figés ; la
distribution et les autres parties du PLAN restent ouvertes. Prochaine étape :
vectorisation bitmap, réalisée ci-dessous.

### Ajout courant : vectorisation bitmap en tracés éditables

Calque > Vectoriser le bitmap propose un dialogue avec comparaison source/résultat,
mode monochrome ou couleur, seuil/inversion, couleur, palette RGBA, transparence,
résolution, petites zones, simplification et arrondi. Les réglages restent en
brouillon ; annulation, Échap et réponses retardées sont vérifiés. Une révision
du document empêche d’appliquer un calcul périmé.

Le résultat est un nouveau calque de chemins fermés en pair-impair, avec nœuds,
îles, trous et poignées de Bézier éditables. Le bitmap reste conservé, masqué
par défaut ou visible au choix. Position, opacité, fusion et effets sont repris ;
la sélection peut limiter le résultat, avec sa progressivité. Undo/redo couvre
l’ensemble en une étape. Les budgets de calcul refusent une image trop complexe
sans résultat partiel ; la grille maximale est 1024 pixels sur le grand côté.

Le projet reste **v16**, puisque le résultat utilise les tracés déjà pris en
charge. Une vraie archive du jalon des polices est relue et sauvegardée avec
ses styles et ses pixels conservés. L’export SVG exclut le bitmap source masqué ;
SVG optimisé et PDF conservent la géométrie produite. Les photographies ne sont
pas reconnues en objets ; palette, résolution, simplification et arrondi sont
des approximations à régler avec l’aperçu. L’enregistrement en actions/lots
reste exclu pour cette commande avec révision explicite.

Validation : **151 tests du cœur et deux corpus externes**, **443 contrôles
Windows**, dont **24 nouveaux**, **19 comparaisons PDF** et **15 inspections
structurelles PDF**. Huit nouveaux tests couvrent notamment les **511 motifs
binaires 3 × 3**. L’aperçu couleur, le résultat appliqué, le SVG et son optimisation
ont un écart nul dans la référence indépendante avec trous et semi-transparence.
Les deux nouveaux PDF gardent les chemins, dont 65 courbes dans l’arrondi,
sans image XObject ; leurs moyennes RGB sont 0,21431 et 0,25686, avec écarts
d’arête et aplats identiques au-delà de la bande d’un pixel.

Voir VECTORISATION.md. Les preuves et sources sont figées dans
outputs/Progression-vectorisation. Les distributions existantes restent figées.
À ce jalon, le plan comportait encore **14 grands lots ouverts**, à des degrés différents
d’avancement à la date de ce jalon ; le suivi courant ci-dessus passe à dix
lots après la réalisation de III.1 à III.4 et ajoute PFV en complément.

## Livrable autonome du 26 octobre

**Prisme-apercu-2026-10-26.zip** contient un EXE autonome, les exemples v11,
le SDK et les documents. L’interface est embarquée : aucun serveur de
développement à lancer. Le numéro interne reste 1.4.0 ; le nom « aperçu »
distingue ce code des anciens paquets 1.4 et d’une future livraison finale.
WebView2 reste requis sur Windows.

Le binaire optimisé passe **299 contrôles Windows**, trois contrôles de ressources
embarquées et les quatre comparaisons PDF. Le contrôle direct d’un module
TypeScript nécessite Vite et n’est pas compté sur ce portable ; le clic réel
sur la région découpée y est bien vérifié. Son empreinte et ses rapports propres
sont joints. Les tests du jalon de développement correspondant étaient au nombre de 300.
Les anciens paquets 1.4 sont conservés avec leurs empreintes d’origine.
Cet aperçu ne clôture pas les exigences ci-dessous ni les essais d’installation
et de mise à niveau : aucun nouvel installateur final n’est annoncé.

## Non terminé

| Domaine | Reste à réaliser ou à prouver |
|---|---|
| Plugins PFL | Comparaison dans PhotoFiltre, PFL tiers x64 publié sans adaptation, et pack original de cinq extensions prévu en partie IV. |
| Vectoriel avancé | Motifs complexes et édition graphique du carreau/des découpes/masques/sources de filtres, couverture PDF des peintures répétées/rayons de foyer et fusion additive, validation de distribution, import du texte SVG en caractères éditables et couverture des variantes PFV hors du contrat observé. Vectorisation et polices variables/styles sont réalisés en développement, avec limites documentées. |
| Couleur | Pipeline ICC et 16 bits par canal de bout en bout ; profils et conversions dans projets, outils et formats. |
| Grandes images | Tuiles, traitements GPU wgpu réellement employés, repli CPU et mesures au-delà des limites actuelles. |
| Aperçus avancés I.3 | Les réglages bitmap et aperçus spécialisés sont réalisés. Interruption interne de tous les algorithmes, traitements tuilés/GPU et extensions aux plugins selon capacités restent à couvrir. |
| Interface PhotoFiltre | Matrice précise des commandes/options et contrôles sur tailles de fenêtres/DPI ; réglages et dialogues encore à rapprocher. |
| Emplacements inactifs | Pinceaux avancé/artistique, déformation, explorateur d’images, impression et scanner. |
| Fonctions supplémentaires | Les quatre lots III sont réalisés. Les nouveautés proposées F21/F22 (calques de réglage et masques de calque rééditables) restent distinctes et ouvertes. |
| Finition | FR/EN complets, aide intégrée, mises à jour signées avec vraie configuration de publication. |
| Distribution finale | Parcours d’installation/mise à niveau, associations, cas d’erreur et revalidation consolidée de tout le périmètre. |

Le code reste principalement RGBA8 avec limites documentées de dimensions et
mémoire. Les commandes inactives ne sont pas comptées comme implémentées. Les
succès d’un composant ou d’une fixture ne prouvent pas l’ensemble d’une phase.

## Trente propositions

[AMELIORATIONS_30.md](AMELIORATIONS_30.md) contient exactement dix correctifs,
dix améliorations de fonctions existantes et dix nouvelles fonctions proposées.
Chaque entrée indique un effet concret ou une preuve attendue. Cette liste
n’enlève aucune exigence ouverte du plan actuel et ne signifie pas qu’elle est implémentée.
Les deux propositions initiales déjà corrigées (champs numériques et images
externes) ont été remplacées dans la liste courante pour garder trente pistes
encore ouvertes, dont exactement dix correctifs, dix améliorations et dix nouveautés.
