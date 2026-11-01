# Prisme — nouveau plan de poursuite
Date du bilan et de la réorganisation : 25 octobre 2026.

Ce plan suit la demande : **terminer le plan précédent → rapprocher fortement
l’agencement de PhotoFiltre 11 → améliorer l’expérience → enrichir par de vraies
extensions .pfl**. Il remplace le plan de travail, sans retirer ses exigences ouvertes.

- Bilan de départ : [BILAN.md](BILAN.md).
- Ancien plan intégral, conservé avant réécriture : [PLAN_PRECEDENT_2026-10-25.md](PLAN_PRECEDENT_2026-10-25.md).
- Preuves techniques et détail d’exécution : [EXECUTION.md](EXECUTION.md), [IMPLEMENTATION.md](IMPLEMENTATION.md).
- Référence locale PhotoFiltre : ../photofiltre ; interopérabilité : [INTEROP.md](INTEROP.md).

Les cases ouvertes sont du travail à faire. Les fonctions déjà testées en
développement ne sont pas automatiquement présentes dans les installateurs 1.3.
Chaque jalon se clôt par des preuves sur une application Windows réelle et une
mise à jour des limites documentées. Aucun pourcentage global ne remplace cette vérification.

## Partie I — Finir intégralement le plan précédent

Point de départ conservé : phases 1 et 2 implémentées dans leur périmètre
documenté ; phase 3 partielle ; phase 4 partielle ; phase 5 ouverte.
Les acquis ne seront pas réécrits sans nécessité, mais restent couverts par la régression finale.

### I.1 — Terminer la phase 4 : hôtes de plugins

**Déjà implémenté en développement :** SDK Prisme DLL C ABI + manifeste,
scripts Rhai, paramètres, processus hôte séparé, délai, gestion des erreurs,
sélection, alpha, undo, actions et lots. Rapport natif : 16 contrôles réussis.
Le jalon 1.4 livre ces acquis avec les hôtes 8BF et PFL ; les trois payloads
portable/NSIS/MSI ont chacun réussi 214 contrôles. Voir EXECUTION.md.

- [x] Raccorder le cœur 8BF présent dans vendor/eightbf-core au processus isolé et à l’interface (développement).
- [x] Découvrir les filtres 8BF et leurs ressources PiPL, choisir un filtre parmi plusieurs
      entrées, afficher ses informations, son dialogue et son « À propos ».
- [x] Respecter sélection, alpha et modes pris en charge ; restituer annulation utilisateur,
      erreurs et plantages sans modifier le document. Vérifier les paramètres/presets,
      leur persistance et les possibilités réelles de rejeu en actions/lots.
- [x] Exécuter une fixture indépendante du code d’intégration et au moins un filtre
      8BF tiers de provenance/licence documentée. Le cœur compilé et ses 145 tests
      ne suffisent pas à cocher l’hôte.

Preuves 8BF en développement : work/phase4-eightbf-result.json, 32 contrôles
réussis, dont Filter Foundry 1.7.0.25 original x64, 3 072 pixels comparés,
préréglage, action et lot rejoués dans de nouveaux processus. Voir 8BF.md pour
le contrat et les limites. Les preuves PFL et de distribution sont détaillées ci-dessous.
- [x] Cartographier et vérifier les 49 callbacks PFL (27 + 5 + 7 + 10) à partir de PLUGIN_API.md,
      analysis/pfstruct.asm et du SDK de référence. Documenter signatures, offsets,
      tailles, alignement, conventions d’appel, encodage et propriété de la mémoire.
- [x] Implémenter l’hôte PFL Unicode x64 : découverte, Compatibility/Header/Struct,
      informations et menus, Glyph/About si présents, Execute, dialogues et API DBmp
      utile aux plugins retenus. La DLL s’exécute dans un processus séparé.
- [x] Formaliser le sous-ensemble PFL compatible : opérations et versions d’API acceptées,
      dimensions, canaux, sélection, calques et callbacks disponibles. Signaler les
      fonctions VCL non prises en charge avant une application incompatible.
- [ ] Tester un vrai plugin PFL existant et autorisé, indépendant du futur pack Prisme,
      ainsi qu’une fixture contrôlée. Vérifier erreurs, annulation, délai, crash,
      résultat invalide et appels non pris en charge ; l’éditeur reste utilisable.
- [x] Relancer les tests du SDK Prisme, scripts, 8BF et PFL sur les binaires empaquetés ;
      distribuer les exemples et notices nécessaires.

Preuves PFL en développement : work/phase4-pfl-result.json, 46 contrôles réussis,
46 callbacks exercés avec succès ; ShowForm, SetImageDPI et SetApplicationState
refusés explicitement. DBmp GDI réels, sélection/alpha, dialogues Win32 modaux,
annulation/undo, import, export, plantage et délai testés. La référence DemoC
du SDK officiel a été portée en x64 avec son algorithme Affiche inchangé :
3 072 pixels comparés et sélection progressive vérifiée. Ce port n’est pas un
plugin tiers x64 publié sans modification. La comparaison du même PFL dans
PhotoFiltre 11 reste à faire. Voir PFL.md et PFL_ABI.md.

Validation du jalon 1.4 : 214 contrôles sur chacune des trois variantes réellement
exécutées, avec interface embarquée et exemples provenant du dossier livré.
Les 20 ressources du SDK/notices sont identiques dans portable, NSIS et MSI.
L’installation complète et la mise à niveau restent à vérifier en I.4 ; les
essais actuels NSIS/MSI portent sur les payloads extraits des paquets.

**Critère de fin :** les trois familles de plugins sont exécutées réellement et
documentées séparément. La compatibilité PFL demeure expérimentale et bornée à son
sous-ensemble démontré ; aucun support universel, VCL ou 32 bits n’est supposé.
Les tests d’une ABI Prisme ne constituent pas une preuve de compatibilité PFL.
La partie IV constituera ensuite un test de confirmation plus large de cette phase.

### I.2 — Terminer la phase 3 : vectoriel avancé

Complément du 27 octobre : import PFV réalisé sur dix références réelles et une
fixture originale, voir [PFV.md](PFV.md). I.2 reste ouvert pour ses autres critères.

- [ ] Groupes éditables : créer, dissocier, imbriquer, sélectionner les enfants,
      déplacer/transformer le groupe et conserver ses styles.

  Implémentation de développement réalisée après 1.4 : six tests du cœur et
  15 contrôles natifs, dont déplacement/redimensionnement, édition d’un enfant
  et d’un nœud dans deux groupes transformés, undo/redo et projet v7. Dissociation
  sans rastérisation, rendu comparé exact dans la fixture native. Groupement
  limité aux formes consécutives d’un même parent ; dissociation d’un groupe
  translucide refusée explicitement pour ne pas distribuer son opacité et altérer
  les recouvrements. Voir GROUPES.md. La case complète reste ouverte jusqu’au
  traitement des styles complexes ci-dessous et à la validation de distribution.

  Complément réalisé ensuite en développement : import des groupes SVG avec
  opacité/fusion/isolation, 17 modes de fusion et édition de leurs enfants ;
  six tests Rust et 19 contrôles natifs. Refus explicite de dissocier un contexte
  non neutre qui changerait le rendu ; identités récursives corrigées à l’import
  et à la duplication. Voir SVG_GROUPES_STYLES.md. La distribution reste ouverte ; les filtres sont ensuite traités au jalon
  v13 décrit ci-dessous, puis feImage et ses sources au jalon v14. Les découpes puis
  les masques peints ont été réalisés ensuite ; voir SVG_MASQUES.md.

- [ ] Dégradés et motifs SVG éditables ; unités locales/globales et transformations.

  Dégradés linéaires/radiaux réalisés en développement après 1.4 : fond/trait,
  texte, points et alpha, prolongement, matrice et conversion des repères local/
  parent conservant le rendu. Import SVG transformé en paramètres éditables,
  projet v8 puis v9 et export SVG/PDF. Quatre tests du cœur et 15 contrôles natifs.
  Motifs simples réalisés ensuite : carreau et contenu vectoriel éditables,
  aperçu natif du carreau en brouillon, repères/matrice, import et persistance v9 ;
  trois tests du cœur et 20 contrôles natifs. Motifs récursifs et contenu SVG complexe,
  édition graphique des nœuds dans un carreau et validation de distribution
  restent ouverts. Le PDF est comparé visuellement et numériquement ; répétition/
  miroir des dégradés et rayon de foyer non nul sont refusés avec un message et
  restent à prendre en charge dans ce format. Voir PEINTURES.md et MOTIFS.md.

- [ ] SVG complexes : images intégrées, découpes, masques, groupes avec opacité/fusion
      et effets, avec règles explicites pour les éléments devant rester un bloc.
      Un élément non pris en charge ne doit jamais disparaître silencieusement.

  Opacité, fusion et isolation sont maintenant importées en groupes éditables.
  Images PNG/JPEG/GIF/WebP/SVG intégrées et découpes vectorielles sont maintenant
  éditables, avec remplacement, interpolation, nœuds, brouillon, undo et projet v11.
  Masques peints alpha/luminance réalisés ensuite : contenus, peintures, images,
  groupes et intersections éditables, aperçu en brouillon, sélection des pixels
  visibles, undo et projet v12. Les masques éliminés par la normalisation restent
  en bloc avec refus explicite. Seize familles de filtres/effets sont ensuite réalisées en chaînes éditables
  (projet v13), avec paramètres, entrées, repères, régions, aperçu, sélection,
  annulation et exports. Le PDF prépare les seuls sous-arbres filtrés avec le
  moteur natif ; les autres objets restent vectoriels. feImage est ensuite réalisé
  au jalon v14 : sources bitmap et références vectorielles locales autonomes,
  contenu éditable, remplacement, ajustement, aperçu, undo et migration v13.
  Les sources manquantes ou incompatibles gardent le bloc entier avec diagnostic.
  Voir SVG_FEIMAGE.md et SVG_FILTRES.md. Voir SVG_MASQUES.md et
  SVG_IMAGES_DECOUPES.md ; l’ensemble de cette
  exigence et sa validation dans la distribution finale restent ouverts.
- [ ] Texte sur tracé : choix du chemin, décalage, orientation, édition du texte et
      conservation des liens ; distinguer texte éditable et conversion en contours.

  Réalisé en développement au jalon v15 : guide lié du même calque, choix du
  contour, départ en pixels/pourcentage, alignement, écart et sens. Texte,
  police et contenu restent éditables ; détachement et conversion en contours
  sont distincts et annulables. Neuf tests du cœur et 26 contrôles Windows
  couvrent aussi nœuds, groupes, duplication/import, sélection et migration
  d’un véritable projet v14. SVG et PDF sont comparés, avec écarts d’arête
  publiés. Voir TEXTE_SUR_TRACE.md. L’import SVG décomposé produit explicitement
  des contours ; la distribution de ces acquis reste ouverte. La case globale
  reste ouverte jusqu’à la validation de distribution.
- [ ] Polices variables et styles de texte : axes, valeurs, styles réutilisables,
      rendu et remplacement explicite d’une police absente.

  Réalisé en développement au jalon v16 : catalogue installé, axes fvar réels,
  valeurs explicites et taille optique automatique, aperçu natif en brouillon,
  styles par copie et bibliothèques importables/exportables. Police absente
  signalée sans réécrire son nom ; remplacement explicite annulable. Sept tests
  du cœur et 23 contrôles Windows comparent aussi les glyphes à des références
  FontTools originales et à un SVG textPath indépendant. SVG conserve les axes ;
  le PDF prépare les glyphes variables en contours depuis une copie. Voir
  POLICES_STYLES.md. Le projet relit v1–v16 et migre une vraie archive v15.
  La distribution de ces acquis reste ouverte ; la case globale est conservée.
- [ ] Vectorisation bitmap : réglages monochrome/couleur, seuil/palette,
      simplification et lissage ; produire des tracés réellement éditables.

  Réalisée en développement : dialogue Calque > Vectoriser le bitmap, aperçu
  en brouillon, grille bornée, seuil/inversion, palette RGBA, petits détails,
  simplification et arrondi cubique. Nouveau calque, bitmap conservé, sélection
  douce et propriétés du calque reprises, une étape annulable. Huit tests du
  cœur couvrent notamment les 511 motifs binaires 3 × 3 ; 24 contrôles Windows
  vérifient édition des nœuds, pixels, révisions, anciens projets et exports.
  Le résultat utilise les tracés v16 existants, sans nouveau format de projet.
  Voir VECTORISATION.md. La distribution de ces acquis reste ouverte.
- [x] Import PFV : établir le format sur des références indépendantes,
      importer géométrie et propriétés reconnues ; refuser proprement les variantes inconnues.
- [ ] Étendre le projet .prisme, les migrations, undo/redo, hit-testing et exports SVG/PDF
      à ces structures. Tester aussi les anciens projets.

**Critère de fin :** corpus de documents simples et complexes, édition après
sauvegarde/réouverture, export et comparaison de rendu ; les écarts mesurés sont
expliqués. Les booléens et la plume déjà livrés continuent de fonctionner.

### I.3 — Terminer la phase 5 : couleur, performances et aperçus

- [ ] Définir puis réaliser le pipeline 16 bits par canal, sa précision interne,
      les conversions ICC, l’alpha et la gestion des profils d’entrée/document/sortie.
- [ ] Conserver profils et profondeur dans les formats compatibles et dans .prisme.
      Afficher les conversions nécessaires pour les outils/formats/plugins limités à 8 bits.
- [ ] Tester niveaux, courbes, mélanges, effets, sélections, imports/exports et round-trips
      avec un corpus de dégradés et profils indépendants ; éviter les pertes silencieuses.
- [ ] Mettre en place le stockage et le traitement par tuiles, des limites mémoire
      explicites et un historique adapté aux grandes images.
- [ ] Utiliser réellement wgpu pour des traitements/rendus identifiés ; repli CPU
      fonctionnel, comparaisons CPU/GPU, gestion d’un GPU indisponible ou perdu.
- [ ] Produire des aperçus de filtres en temps réel : travail asynchrone, annulation
      des anciennes demandes, pas de résultat périmé, comparaison avant/après.
      Aucun changement de pixels ni entrée d’historique avant « Appliquer ».
- [ ] Mesurer les temps de calcul et la mémoire sur un corpus défini, dont des
      images dépassant la limite actuelle de 16 Mpx ; publier machine et mesures.
      Ne relever les limites qu’après validation des chemins tuilés.

**Critère de fin :** images de grande taille réellement éditées, aperçus réactifs
et annulables, travail 16 bits et ICC vérifié de bout en bout. Une option GPU ou
une étiquette « 16 bits » sans pipeline effectif ne clôt pas cette étape.

### I.4 — Terminer la phase 5 : finition et distribution

- [ ] Internationaliser tous les parcours FR/EN : menus, panneaux, dialogues, erreurs,
      aide et textes de plugins fournis ; mémoriser la langue et vérifier les débordements.
- [ ] Aide intégrée : outils, raccourcis, projets, formats, limites et installation des plugins.
- [ ] Mises à jour automatiques signées : configuration réelle de publication,
      génération/conservation des clés, vérification cryptographique et installation.
      Vérifier aussi refus de signature invalide, réseau indisponible et reprise après échec.
      La clé publique et le canal de distribution devront correspondre au vrai déploiement.
- [ ] Rejouer phases 1/2/3/4, corpus externes et parcours natifs ; reconstruire
      EXE NSIS, MSI et portable avec dépendances, ressources, notices et documentation.
- [ ] Vérifier les fichiers installés, premier lancement, mise à niveau, sauvegarde
      des anciens projets et correspondance entre version, source et rapports.

**Critère de fin de partie I :** toutes les exigences ouvertes de l’ancien plan
ont une implémentation et une preuve. Un jalon intermédiaire reste permis, sans
annoncer prématurément la clôture du plan ni de la phase 4.

## Partie II — Ressembler fortement à PhotoFiltre 11 par l’agencement

Objectif : retrouver les commandes aux endroits attendus, y compris leurs options
contextuelles. Le placement général existant est un point de départ ; il ne suffit
pas à considérer cette partie terminée.

### II.1 — Relevé précis et correspondance des commandes

- [ ] Compléter le relevé des formulaires locaux TMAINFORM/TOUTILSFORM et des dialogues.
      Construire une matrice : commande PhotoFiltre → position → équivalent Prisme
      → disponible/à développer → raccourci → preuve de placement.
- [ ] Comparer avec la fenêtre PhotoFiltre 11 en fonctionnement dès qu’elle est
      pilotable. Le relevé actuel provient des formulaires, sans comparaison visuelle directe.
- [ ] Conserver des captures de référence autorisées pour le travail de comparaison,
      sans embarquer les ressources propriétaires dans Prisme.

### II.2 — Fenêtre principale et outils

- [ ] Reprendre ordre, regroupements et séparateurs des menus ; deux barres en haut :
      commandes générales puis réglages/filtres rapides.
- [ ] Calques à gauche ; couleurs premier plan/arrière-plan en haut à droite ;
      palette d’outils en dessous ; options de l’outil sous cette palette ;
      document central ; informations et zoom dans la barre d’état.
- [ ] Rapprocher densité, alignement, dimensions et espacement des contrôles.
      Garder les outils vectoriels supplémentaires regroupés après les outils de référence.
- [ ] Reproduire la grille fonctionnelle suivante et ses sous-options :

| Rangée | Gauche | Centre | Droite |
|---|---|---|---|
| 1 | Sélection | Gestion des calques | Déplacement de la vue |
| 2 | Pipette | Baguette | Ligne |
| 3 | Remplissage | Aérographe | Gomme |
| 4 | Pinceau | Pinceau avancé | Tampon |
| 5 | Flou | Doigt | Retouche |
| 6 | Déformation | Pinceau artistique | Tube |

- [ ] Pour chaque outil, rapprocher l’ordre et la position des réglages : forme,
      taille, pression/intensité, tolérance, contour progressif, alignement du tampon,
      dimensions de sélection, options de ligne et source du tube selon l’outil.
      Les réglages propres à Prisme restent clairement identifiables.
- [ ] Rapprocher emplacement et contenu des boîtes taille d’image, zone de travail,
      rotation, couleur, texte, niveaux/courbes et propriétés de calque.
- [ ] Permettre un espace de travail « PhotoFiltre » par défaut et mémoriser tailles,
      panneaux visibles et état des barres ; proposer « Réinitialiser l’atelier ».

### II.3 — Combler les emplacements encore inactifs

- [ ] Pinceau avancé : formes et dynamique d’application ; pinceau artistique :
      plusieurs rendus distincts ; déformation : pousser, dilater/rétrécir et restauration.
      Chaque geste respecte sélection, alpha et undo.
- [ ] Explorateur d’images intégré : vignettes, ouverture, navigation clavier et glisser-déposer.
- [ ] Impression : aperçu, format papier, orientation, échelle/marges et sortie
      par le dialogue système ; vérifier le résultat avec une imprimante PDF.
- [ ] Acquisition scanner : intégration Windows disponible, choix de périphérique,
      import du résultat et annulation. Le test sur matériel reste une validation
      distincte à consigner, sans le déclarer passé faute de scanner.
- [ ] Aucune commande ne devient active avant que son parcours fonctionne.

**Critère de fin de partie II :** matrice de correspondance remplie et comparaison
côte à côte des écrans principaux et options par outil. Vérifier fenêtres normales
et maximisées, petite fenêtre supportée, zoom Windows 100/125/150 %, navigation
clavier et panneaux déroulants. Les options essentielles restent accessibles.
Les icônes et l’implémentation restent celles de Prisme.

## Partie III — Améliorations de l’expérience et de la praticité

### III.1 — Vrais aperçus de calques, priorité de cette partie — réalisé

- [x] Remplacer les icônes actuelles par des miniatures du contenu réel :
      bitmap, vectoriel, texte, groupes, transparence et effets.
- [x] Montrer le calque isolé dans sa miniature ; conserver un badge de type,
      l’état de visibilité et un repère clair pour le calque actif.
- [x] Mise à jour après peinture, transformation, effet, undo/redo et réouverture.
      Cache par révision et calcul asynchrone ; ignorer les rendus périmés.
- [x] Taille des vignettes réglable, damier lisible et rendu net aux différents DPI.
      Limiter le travail aux éléments visibles d’une longue liste.
- [x] Si des masques non destructifs sont ajoutés, afficher leur propre miniature
      et l’état lié/délié ; ne pas présenter les masques appliqués aux pixels comme éditables.

**Validation :** le rendu de chaque miniature correspond au bon calque après toutes
ces opérations ; aucune miniature ancienne n’est affichée après changement de document.

### III.2 — Manipuler les calques plus vite — réalisé

- [x] Glisser-déposer pour réordonner avec indicateur de destination ; double-clic
      de renommage en ligne, duplication et menu contextuel complet.
- [x] Sélection multiple, opérations groupées et verrouillages : position, pixels,
      alpha ; définir le comportement des outils sur un calque verrouillé.
- [x] Afficher un calque seul temporairement, retrouver la visibilité précédente,
      filtrer/rechercher les calques et simplifier l’accès aux effets.
- [x] Une action utilisateur cohérente donne une seule étape d’historique ;
      enregistrement, réouverture et undo conservent les nouveaux états.

### III.3 — Rendre les gestes et réglages plus pratiques — réalisé

- [x] Zoom ancré sous le pointeur, déplacement temporaire avec Espace, ajustement
      à la fenêtre et accès rapide à 100 % ; curseur de pinceau fidèle à sa taille.
- [x] Valeurs numériques au clavier, bornes et unités explicites, remise à zéro
      d’un réglage et préréglages des outils conservés entre sessions.
- [x] Recherche de commande avec raccourcis visibles, commandes récentes et favoris.
- [x] Palette des couleurs récentes, échange premier/arrière-plan et saisie couleur
      pratique ; pipette utilisable depuis les dialogues compatibles.
- [x] Utiliser l’aperçu non destructif de I.3 dans les réglages et plugins qui le
      permettent : avant/après, réinitialisation, Appliquer/Annuler prévisibles.
- [x] Panneaux redimensionnables/repliables, messages d’erreur utiles et discrets,
      focus clavier visible ; éviter les dialogues répétitifs lors d’actions courantes.
- [x] Traitements longs : progression, annulation quand techniquement possible,
      aucune validation tardive sur un autre document ou calque.

### III.4 — Continuité du travail — réalisé

- [x] Fichiers récents avec vignettes et chemins complets accessibles ; gestion
      explicite des fichiers manquants et du document modifié.
- [x] Sauvegarde de récupération périodique et restauration après interruption,
      avec remplacement atomique et nettoyage après sauvegarde normale.
- [x] Ouvrir plusieurs documents dans des onglets, chacun avec son historique,
      son zoom et ses sélections ; choix clair de la cible des plugins et lots.
- [x] Vérifier fermeture, ouverture pendant un calcul, récupération, bascule
      de document et persistance des préférences.

**Critère de fin de partie III :** parcours quotidiens complets avec calques nombreux,
clavier/souris, annulation, récupération et plusieurs documents ; mesurer l’absence
de blocage et les mises à jour des aperçus. Les améliorations ne doivent pas
déplacer les commandes essentielles de l’atelier PhotoFiltre défini en partie II.

Validation du 27 octobre 2026 : les quatre lots III.1 à III.4 sont réalisés
avec leurs contrats et limites dans [EXPERIENCE.md](EXPERIENCE.md). La condition
sur une nouvelle miniature de masque de calque est sans objet ici : ce jalon
n'ajoute pas de masque de calque bitmap rééditable ; F22 reste ouverte. La pipette
des dialogues dépend de EyeDropper dans WebView2. Un aperçu CPU déjà lancé peut
finir sur sa copie après son annulation ; aucun résultat tardif n'est appliqué.
Les plugins conservent les capacités d'aperçu/annulation de leurs hôtes.
Les futurs GPU/tuiles et une prévisualisation universelle ne sont pas cochés en I.3.
Preuves : 34 contrôles spécifiques plus cinq avec redémarrage réel, intégrés au
nouveau dossier de preuves, avec régression du reste de l'application.

## Partie IV — Enrichir l’application avec un pack original de vrais .pfl

Cette partie est retenue : elle apporte des fonctions supplémentaires et éprouve
l’hôte PFL avec du code plugin distinct du cœur de Prisme. Elle commence après
la première validation PFL de I.1 ; elle ne sert pas à en repousser l’implémentation.

### IV.1 — SDK et pack PFL séparés

- [ ] Fournir un SDK PFL x64 documentant le sous-ensemble réellement pris en charge,
      des exemples compilables, commandes de build et sources sous licence libre.
- [ ] Compiler de vrais fichiers .pfl avec les exports et structures PhotoFiltre.
      Aucun renommage d’une DLL utilisant prisme_filter_v1 en .pfl.
- [ ] Utiliser d’abord l’API DBmp ; ajouter des callbacks de calques seulement
      après validation indépendante de leurs signatures et de leur comportement.
- [ ] Installation depuis un dossier, catalogue lisible, nom/version/auteur,
      activation et retrait ; une extension absente ne bloque pas le démarrage.
- [ ] Séparer les sources du pack et celles de l’hôte, avec communication par
      l’ABI publique ; aucune entrée spéciale réservée aux plugins du pack.

### IV.2 — Fonctions proposées pour le premier pack

| Extension originale | Apport et réglages principaux | Ce qu’elle vérifie |
|---|---|---|
| Grain argentique | Grain luminance/couleur, taille, intensité, graine reproductible | Lecture/écriture DBmp, dialogue, alpha, répétabilité |
| Palette pixel-art | Palette utilisateur/extraite, nombre de couleurs, tramage réglable | Réduction de couleurs distincte du mode indexé du document, sélection, paramètres |
| Aberration chromatique | Décalage radial des canaux, centre et intensité | Accès voisinage, interpolation, bords et transparence |
| Texture raccordable | Raccord horizontal/vertical, largeur de fusion et aperçu répété | Traitement global, dimensions, annulation et aperçu |
| Carte normale | Conversion d’une hauteur en normales, force, inversion des axes | Calcul sur voisins, convention des canaux, formats de sortie |

Ce sont de nouvelles fonctions à réaliser, pas des filtres déjà livrés.
La réduction de palette produit ici une image traitée : elle ne doit pas
prétendre convertir le modèle complet du document en mode indexé.

- [ ] Chaque plugin offre un aperçu, des valeurs par défaut, une réinitialisation,
      Appliquer/Annuler et une description courte de son usage.
- [ ] Préserver sélection et transparence selon un contrat documenté ; définir
      les conversions pour documents 16 bits/profils ICC.
- [ ] Intégrer actions/lots et préréglages seulement par les capacités publiques
      effectivement supportées ; signaler clairement une fonction non rejouable.

### IV.3 — Épreuve de confirmation de la phase 4

- [ ] Exécuter chaque .pfl dans Prisme et PhotoFiltre 11 x64, sur les mêmes images
      et réglages. Comparer dimensions, alpha et pixels ; définir les tolérances
      avant comparaison pour les traitements interpolés.
- [ ] Vérifier la matrice : sans sélection/avec sélection, transparence, calque
      décalé, Annuler, Appliquer puis undo/redo, sauvegarde/réouverture, paramètres,
      actions/lots si compatibles, appel About et erreurs.
- [ ] Ajouter des extensions de test avec erreur, délai, crash et sortie invalide.
      Vérifier l’isolation et le document inchangé après échec.
- [ ] Conserver séparément le test d’un PFL tiers de I.1 : deux composants développés
      ensemble ne suffisent pas à démontrer l’interopérabilité.
- [ ] Si la fenêtre PhotoFiltre ne peut pas être pilotée, garder la comparaison
      inter-hôtes ouverte avec protocole manuel et résultats à renseigner.
      Les tests Prisme seuls ne permettent pas de cocher cette comparaison.
- [ ] Livrer pack, sources, SDK, limites de compatibilité et rapport par plugin,
      puis reconstruire et vérifier EXE/MSI/portable finaux.

**Critère de fin de partie IV :** cinq extensions utiles réellement distribuées
et exécutées via l’ABI PFL publique ; preuves inter-hôtes et de robustesse.
La conclusion doit nommer précisément les versions et callbacks compatibles.

## Ordre d’exécution et suivi

Le plan comporte désormais **10 lots de niveau I.1 à IV.3 encore ouverts**.
Les quatre lots III.1 à III.4 sont réalisés ; les dix autres gardent des degrés
d’avancement différents. Un jalon fonctionnel ne ferme pas à lui seul un lot :
la vectorisation et PFV appartiennent tous deux à I.2, qui garde aussi ses critères
de couverture et de distribution. Ce nombre ne représente pas un nombre fixe
de sessions de travail restantes.

1. I.1 : acquis Prisme/Rhai/8BF/PFL livrés et revalidés dans le jalon 1.4 ;
   contrôle croisé PFL et plugin tiers x64 publié restent ouverts.
2. I.2 : achever les critères vectoriels restants ; PFV a maintenant ses preuves.
3. I.3 puis I.4 : précision/couleur, tuiles/GPU/aperçus, finition et livraison de fin d’ancien plan.
4. II : réaliser l’agencement détaillé et les options manquantes.
5. III : quatre lots réalisés au jalon expérience du 27 octobre ; conserver leur régression.
6. IV : pack PFL et validation croisée, puis livraison consolidée.

Le prochain travail concret est de poursuivre les exigences vectorielles encore
ouvertes de I.2 et leur distribution. PFV est réalisé dans son contrat observé.
La vectorisation a ses preuves de développement dans VECTORISATION.md.
Les sources feImage, le texte sur tracé et les polices/styles
ont leurs preuves de développement dans SVG_FEIMAGE.md, TEXTE_SUR_TRACE.md et
POLICES_STYLES.md. Images et découpes ont leurs preuves dans SVG_IMAGES_DECOUPES.md.
Les peintures ont leurs preuves dans
PEINTURES.md et MOTIFS.md. Les groupes sont décrits dans GROUPES.md et
SVG_GROUPES_STYLES.md. Le contrôle PFL dans PhotoFiltre reste ouvert avec son protocole
manuel ; l’hôte DBmp et les 46 contrôles PFL sont revalidés dans les trois variantes 1.4.
I.2 avance sans attendre indéfiniment une fenêtre de référence absente
et sans cocher sa validation croisée. Pour chaque jalon,
EXECUTION.md doit consigner : modification, binaire/version, tests et rapports,
limites restantes, puis prochaine action. Le présent document est un plan,
pas une attestation d’exécution des cases ouvertes.

