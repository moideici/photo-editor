# Exécution du plan complet

Le 25 octobre 2026, PLAN.md a été réorganisé à la demande de l’utilisateur :
I. terminer l’ancien plan ; II. rapprocher l’agencement de PhotoFiltre ;
III. améliorer l’expérience ; IV. fournir un pack original de vrais PFL.
L’ancien plan intégral est conservé dans PLAN_PRECEDENT_2026-10-25.md et le bilan
de départ dans BILAN.md. Cette réorganisation documentaire n’ajoute aucune
fonction au binaire. Le tableau suivant suit les exigences de la partie I ;
les parties II et IV restent ouvertes ; la partie III est réalisée au jalon
expérience du 27 octobre, selon le contrat de EXPERIENCE.md.

Objectif conservé : terminer le périmètre convenu, sans remplacer les exigences restantes par
les jalons déjà livrés. Le dernier jalon de distribution vérifié est Prisme 1.4 :
81 tests Rust, 2 corpus externes et 214 contrôles sur chacune des variantes
portable/NSIS/MSI (payloads extraits pour les installateurs). Ce travail constitue
une progression ; il ne prouve pas l’achèvement du plan.

| Exigence restante | État actuel | Preuve nécessaire |
|---|---|---|
| Groupes vectoriels éditables | Base 15 contrôles ; import/style opacité-fusion-isolation et enfants éditables : 19 contrôles supplémentaires | Dissociation d’un contexte non neutre refusée sans perte ; effets/masques/découpes et distribution encore ouverts ; voir GROUPES.md et SVG_GROUPES_STYLES.md |
| Dégradés/motifs et SVG complexes éditables | Dégradés linéaires/radiaux (15 contrôles) et motifs simples avec aperçu natif du carreau (20 contrôles) réalisés en développement après 1.4 | Contenu complexe, édition graphique du carreau, certaines peintures PDF et prochaine distribution restent ouverts ; voir PEINTURES.md et MOTIFS.md |
| Texte sur tracé, polices variables et styles | Réalisés en développement aux jalons v15/v16 | TEXTE_SUR_TRACE.md et POLICES_STYLES.md ; distribution encore ouverte |
| Vectorisation bitmap | Réalisée en développement, tracés v16 existants | VECTORISATION.md ; huit nouveaux tests et 24 contrôles Windows ; distribution encore ouverte |
| Tracés PFV | Réalisés dans le contrat observé : dix vrais fichiers et fixture originale | PFV.md et nouveau dossier de preuves ; variantes inconnues refusées |
| API de plugins Prisme DLL C ABI + manifest | Livrée et revalidée en 1.4 : 16 contrôles par variante avec Rhai | SDK documenté, DLL C réelle, paramètres, sélection, undo et processus isolé |
| Filtres scriptés Rhai/Lua | Rhai livré et revalidé en 1.4 | Exemples, paramètres, erreurs et limites, rendu, sélection, actions et lots |
| Hôte Photoshop 8BF | Livré et revalidé en 1.4 : 32 contrôles par variante, dont Filter Foundry tiers | Limites dans 8BF.md ; compatibilité universelle non annoncée |
| Plugins PhotoFiltre PFL expérimentaux | Sous-ensemble livré et revalidé en 1.4 : 46 contrôles par variante | Comparaison dans PhotoFiltre et plugin tiers x64 publié encore ouverts ; contrat dans PFL.md |
| Grandes images : tuiles et GPU wgpu | À réaliser | Traitement tuilé et GPU réellement utilisés, comparaison CPU et mesure mémoire/performance |
| Aperçu des réglages | Réalisé : copie, temporisation, avant/après, réinitialisation et rejet des réponses tardives | EXPERIENCE.md ; interruption interne des calculs et autres travaux I.3 restent distincts |
| ICC et 16 bits par canal | À réaliser | Pipeline de données et conversion de profil, formats, sauvegarde, filtres et corpus |
| Traductions FR/EN et aide intégrée | À réaliser | Parcours complets dans les deux langues et aide accessible |
| Mises à jour automatiques signées | À réaliser | Signature vérifiée, téléchargement/install et refus d’une signature invalide ; configuration réelle de distribution à auditer |
| Distribution finale | À reconstruire après les extensions | Installateurs, portable, notices, corpus et suite complète sur les binaires finaux |

Les phases 1 et 2 restent à revalider à la fin. Les cases de PLAN.md ne sont
cochées que lorsque l’implémentation et les preuves correspondent au périmètre.

## Preuves après 1.3

- src-tauri/src/plugin_runtime.rs : manifeste, ABI C et exécution Rhai bornée.
- src-tauri/src/plugins.rs : processus séparé, Job Windows (512 Mio/processus et fin des descendants), délai et validation avant application.
- plugin-sdk/ : documentation et exemples originaux ; fixture C compilée avec MSVC x64 dans work/plugin-tests.
- src-tauri/tests/fixtures/plugin_contract.c : erreur, plantage par accès nul, délai et altération de contrat.
- work/phase4-plugins-result.json : 16 contrôles réels dans le binaire de développement, aucun échec JavaScript.
- npm run test:core après intégration 8BF : 79 réussites, deux corpus ignorés lors de cette commande ; les corpus restent à relancer au contrôle final.
- work/native-smoke.cjs, milestone12-ui.cjs et phase3-svg-edit.cjs repassés sur le binaire de développement via le port de test 9225.

Le binaire de distribution 1.3 existant reste l’ancien livrable : il ne contient
pas ces extensions en cours. La distribution finale sera reconstruite après
les autres exigences, sans prendre ce sous-ensemble pour l’objectif complet.

## Préparation 8BF — historique avant raccordement

Le paragraphe de préparation ci-dessous décrit l’état antérieur. Il est dépassé
par la section « Intégration 8BF » à la fin du document.

Le cœur host du projet indépendant eightbf-host est présent dans
vendor/eightbf-core (MIT choisie, licences et provenance conservées). Révision
2a52154f160bb25c9ab2a77c29c7ddaccf0b8258. Les sources host ne sont pas modifiées ;
seuls le manifeste minimal et la racine de crate sont adaptés, pour ne pas intégrer
le serveur HTTP, le CLI ou les images du projet. Dépendance locale de src-tauri.
145 tests unitaires du cœur passent avec cargo test --manifest-path
vendor/eightbf-core/Cargo.toml --lib. Le cargo check de Prisme passe.

Ce cœur n’est PAS encore appelé par le processus hôte ni par l’interface :
la case hôte 8BF reste ouverte. Les types utiles sont :
- host::discovery::read_pe_plugin / read_plugin_path, PluginInfo::key / entry_point.
- host::image::PlaneImage::from_data(width,height,channels,depth,data).
- host::session::{FilterHost,FilterRequest,Hooks,FilterResult} ; run et about.
- host::descriptors::{Descriptor,DescValue} et presets (paramètres opaques).

Étapes restantes sur ce volet : brancher une requête 8BF au processus isolé
existant, découverte PiPL sans charger la DLL dans l’éditeur, choix parmi plusieurs
filtres d’un fichier, paramètres/dialogue/presets et about, actions/lots, puis
exécution d’un filtre réellement compilé et d’une référence indépendante.
La fixture C amont tests/fixtures/testfilter.c ne demande pas le SDK Adobe et
exerce les sélecteurs, suites et descripteurs. Source de travail complète :
work/upstream/8bf-host, même révision ; le fichier C n’est pas encore copié/compilé.
Le binaire pspihost n’a pas été copié depuis PhotoFiltre ; le clone amont alternatif
work/upstream/pspihost est seulement consulté.

La référence officielle PFL est téléchargée pour l’interopérabilité dans
work/pfl-reference.html et .txt ; le SDK local ../photofiltre/PLUGIN_API.md et
analysis/pfstruct.asm restent les références de l’ABI Unicode x64. Les callbacks
PFL et leur preuve d’exécution ne sont pas réalisés.

Le contexte de test courant utilise un binaire de développement distinct, port
CDP 9225, dossier WebView work/webview-plugins ; son PID est dans
work/prisme-plugins.pid et doit être revalidé avant toute action sur le processus.
Le serveur Vite écoute 1420. L’ancien binaire distribué reste indépendant.

Dernier contrôle de progression : travail effectué, non simple attente. Le cœur
8BF vendored a réussi 145/145 tests unitaires (work/eightbf-core-tests.jsonl), et
les notices incluent maintenant les dépendances Rhai/libloading/tempfile ainsi
que la provenance et les licences du composant 8BF. Il n’existe pas de preuve
suffisante pour cocher l’hôte 8BF ni pour déclarer le plan terminé. Les exigences
ouvertes du tableau restent toutes dans l’objectif actif.

## Intégration 8BF — réalisée en développement après le nouveau plan

Le raccordement est maintenant effectif : src-tauri/src/eightbf.rs,
EightBfPanel.tsx, opérations eightBf et enregistrement des paramètres réellement
renvoyés. Le lancement réutilise le Job Windows et les limites de plugins.rs.
Découverte PE/PiPL sans charger la DLL ; plusieurs filtres par fichier ; dialogues,
À propos, préréglages, sélection locale au calque, alpha, undo/redo, actions et lots.

Une régression trouvée avec Filter Foundry a été corrigée dans le cœur : en
présence d’un descripteur de script sans demande d’interface, ne pas appeler
Parameters pour la seule absence de handle. Les filtres déclarant leur terminologie
et renvoyant un descripteur non vide se rejouent par ce descripteur public.
L’adaptation et sa raison sont dans vendor/eightbf-core/UPSTREAM.md.

Preuves : work/phase4-eightbf-result.json, 32 contrôles réussis, aucune erreur JS.
Fixture C issue du composant amont avec licence conservée, treize ressources
PiPL compilées dans un vrai PE x64. Le dialogue natif de fixture se ferme par
son propre WM_TIMER ; l’ancien essai MessageBox avec timer de thread a été
remplacé après dépassement du délai. Erreurs/crash/délai conservent pixels et historique.
Filter Foundry 1.7.0.25 x64 tiers original a été exécuté : 3 072 pixels comparés
exactement à un négatif, puis préréglage, action et lot rejoués dans de nouveaux
processus. SHA et provenance dans le rapport ; binaire tiers non redistribué.

79 tests Prisme réussis (deux corpus externes non rejoués), 145 tests du cœur
8BF réussis. TypeScript/Vite passe. Les 16 contrôles Prisme/Rhai et les parcours
native-smoke, milestone12-ui, phase3-svg-edit repassent. La dernière vérification
8BF est liée au SHA-256 du binaire de développement dans son rapport.

Limites et utilisation : 8BF.md. Les installateurs 1.3 restent antérieurs à ce
travail ; la case de validation empaquetée reste ouverte. La phase 4 n’est pas
terminée : cartographier puis implémenter l’ABI PFL/DBmp est la prochaine étape.

Le processus de test courant est enregistré dans work/prisme-eightbf.pid,
port 9225, profil work/webview-eightbf ; Vite est dans work/vite-eightbf.pid,
port 1420. Vérifier chemin et commande des processus avant toute action sur eux.

## Audit PFL — historique de la préparation

Cette section décrit l’état avant l’intégration ci-dessous ; ses mentions
runtimeVerified=false concernent le rapport de cartographie statique conservé.

Le SDK officiel 1.1 (mars 2005) est disponible dans work/pfl-sdk-reference,
comme référence de travail seulement. Son ordre de callbacks, rapproché des
écritures LEA/store de pfstruct.asm, permet de nommer 49 entrées : 27+5+7+10.
work/map-pfl-callbacks.py produit les rapports et les désassemblages par cible.
Toutes les entrées restent runtimeVerified=false ; aucune exécution PFL annoncée.

Deux hypothèses de PLUGIN_API.md voisin sont corrigées avec preuves : SelRect
est à +0x10 et ImgRect à +0x20 ; le DBmp x64 fait 56 octets, avec pointeurs à
+0x20/+0x28/+0x30. ReleaseHandleDBmp renvoie un HBITMAP et les flags de formats
commencent à 0x10000. Voir PFL_ABI.md et plugin-sdk/pfl/prisme_pfl.h.
La fixture C pfl_abi_probe.c compile avec MSVC x64 /W4 /WX : 22 assertions de
tailles/offsets passent, rapport work/pfl-tests/abi-layout.json. Cela valide les
déclarations, sans prouver le comportement ni l’intégration des callbacks.

Prochaine modification : types Rust et contexte PFL isolé, DBmp BGR24 et tokens
opaques piBitmap, puis découverte/metadata/about/Execute et preuve réelle. Il
faut aussi vérifier les scanlines et la durée de vie des handles contre PF11.
Ce progrès n’enlève aucune exigence des phases I.2–IV du nouveau plan.

## Intégration PFL — réalisée en développement

L’hôte réel est maintenant dans pfl.rs, pfl_abi.rs et pfl_callbacks.rs, avec
PflPanel.tsx et les accès par type dans les menus. Compatibility, Header, Struct,
Language, Glyph, About et Execute sont chargés dans des processus isolés.
Les API 2/3/4 sont bornées à leur préfixe, et le DBmp utilise de vrais DIB/GDI
BGR24, un jeton piBitmap opaque et une propriété explicite des handles.

work/phase4-pfl-result.json : 46 contrôles natifs réussis, aucune erreur JS,
46 callbacks exercés avec succès. Sont vérifiés sélection progressive une
seule fois, calques décalés, alpha inverse, palettes, masques, miniatures,
chaînes, énumération, création/import, export, redimensionnement, undo/redo,
sauvegarde/réouverture, annulation, erreur de contrat, crash, délai, métadonnées
qui plantent, dialogue Win32 modal et À propos. Une nouvelle application
fonctionne après crash/délai. Les appels VCL, DPI modifié, état global et
PrepareDialog modes aperçu/pipette sont refusés avec rollback.

Le transfert du projet complet partage de nouveau les buffers inchangés du
parent (pixels, sélections, EXIF et indices identiques) avant adoption. Un test
effectue un vrai round-trip projet, modifie un calque et vérifie l’identité Arc
des buffers intacts, sans perdre le pixel modifié ni accepter une pile réordonnée.

Référence indépendante : DemoC de P@F, SDK officiel 1.1 (mars 2005), portée
en x64/Unicode/MSVC avec timer de dialogue. L’algorithme Affiche reste identique,
3 072 pixels du résultat sont comparés exactement, puis un masque progressif
est vérifié. Le rapport conserve provenance, adaptations et SHA. Ce n’est pas
un plugin tiers x64 publié sans modification ; sources/ressources officielles
et port restent dans work, sans redistribution. Le seul exemple PFL livré dans
le SDK du projet est le négatif original MIT.

La tentative de contrôle croisé dans la copie de travail de PhotoFiltre 11
n’a fourni aucune fenêtre contrôlable après lancement par Computer Use.
Le contrôle croisé reste ouvert ; aucune comparaison fictive ni contournement
de licence n’est annoncé. Il reste aussi à trouver et tester un PFL tiers x64
publié, puis à revalider tous les hôtes sur les binaires empaquetés.

Régression sur le même binaire de développement SHA-256
37468bd8d6f8fc90071987c9f7bde7ba44446d9e910bc49a695aecf5e918fdbf :
32 contrôles 8BF, 16 Prisme/Rhai, 12 native-smoke, 12 milestone12-ui et 14 SVG
repassent ; TypeScript/Vite passe. Le cœur comporte 81 tests réussis et deux
corpus externes non rejoués à ce jalon. Les installateurs 1.3 restent antérieurs.

Un contrôle Computer Use natif confirme aussi l’ouverture du menu Outils après
les suites de dialogues/erreurs : entrées Prisme, 8BF et PFL visibles, puis
fermeture par Échap. Rapport distinct work/pfl-native-ui-result.json. Le premier
clic sans géométrie a été refusé par l’outil ; une nouvelle capture avec géométrie
a permis la vérification, sans réutiliser l’observation périmée.

Contrat et limites : PFL.md et PFL_ABI.md. Le processus de test courant est dans
work/prisme-pfl.pid, port 9225, profil work/webview-pfl ; Vite dans
work/vite-pfl.pid, port 1420. Vérifier le chemin/commande avant toute action.
Le chantier complet reste actif : I.1 garde ses validations ouvertes et I.2–IV
gardent toutes leurs exigences, sans réduction au sous-ensemble PFL démontré.

## Distribution intermédiaire 1.4 — trois variantes vérifiées

Versions alignées : Cargo, package npm/lock, configuration Tauri et aide affichée
en 1.4.0. EXE NSIS et MSI construits avec frontend embarqué ; portable constitué
du même exécutable release et des ressources déclarées. Les exemples originaux
negative.pfl et invert.dll sont compilés x64, avec sources, manifestes, en-têtes
et notices. Aucun binaire PhotoFiltre, DemoC ou Filter Foundry n’est distribué.

work/package14-result.json : neuf contrôles de structure/contenu, 20 ressources
identiques dans chaque variante. Le payload de chaque paquet est identique au
binaire release, sauf les trois octets du marqueur de type Tauri attendus.
L’extraction MSI utilise les tables File/Component/Directory en lecture seule,
pour reconstruire les chemins réellement installés sans lancer une installation.

work/release14-results/{portable,nsis,msi}/summary.json : **214 contrôles réussis
par variante**, dont 16 Prisme/Rhai, 32 8BF, 46 PFL ; aucune erreur JavaScript.
Les essais chargent les exemples depuis le dossier de la variante, pas depuis
le source, et vérifient le frontend embarqué. Rapports détaillés, captures et SHA
sont conservés par variante. Les ports de contrôle ont été rendus paramétrables
dans les anciens scripts et l’attente du chargement initial a été explicite.

Empreintes SHA-256 des exécutables testés :

- portable/release : 21f7048aad9ca8b064dfc093c02655451dd532c01d782009ab1775a9ff511d02 ;
- payload NSIS : 7e7c8e41b81b61fab490b3cd0a00a8d346bd90c848b127de527c33a54b5e8b78 ;
- payload MSI : 4a17cea8754df417a04d8108bebe693b64aa6a1d8c96226e75bff4ab40e19dc2.

81 tests du cœur repassent. Les deux corpus externes PSD/caméra ont été relancés
et réussissent ; neuf références PSD comparées au rendu psd-tools restent à
un niveau par canal au maximum. HEIC et aperçu RAW sont également testés dans
chaque variante. Les sources et la construction optimisée passent.

Limite précise : ces essais lancent les **payloads extraits** de NSIS/MSI.
Ils ne prouvent pas une installation, mise à niveau, association de fichiers ou
désinstallation complète. Ces parcours restent dans I.4 avec les autres exigences
de distribution finale. La comparaison PFL dans PhotoFiltre et le PFL tiers x64
publié restent ouverts. La case « tests sur binaires empaquetés » de I.1 est
cochée, sans clôturer la phase 4 ni retirer une exigence de I.2–IV.

Prochain travail : groupes vectoriels réellement éditables et persistants,
transformations imbriquées et hit-testing, avant peintures et SVG complexes.

Livrables copiés dans outputs/Prisme-1.4.0 : EXE NSIS, MSI, ZIP portable,
notices et rapports. work/verify-delivery14.py confirme le contenu exact du ZIP
(exécutable et toutes les ressources, sans ajout) et l’identité des installateurs
finaux avec ceux extraits pour les tests. validation-livraison.json et
SHA256SUMS.txt accompagnent les fichiers. Cette livraison est un jalon intermédiaire.

## Groupes vectoriels — développement après la livraison 1.4

Le modèle Shape contient maintenant Group (enfants et dimensions locales) et
une matrice affine facultative sur chaque forme. groups.rs gère la hiérarchie,
les identités, le groupement/dissociation et les limites ; les opérations d’édition,
suppression, conversion, décomposition SVG et booléens retrouvent les enfants
dans leur parent. Le rendu utilise des groupes SVG réels avec opacité isolée.
Les rotations, miroirs et redimensionnements d’image conservent les enfants.

Le sélecteur Forme active est hiérarchique. GroupPanel permet de cocher les
formes d’un même parent, grouper/dissocier et naviguer entre parent/enfant.
Alt+clic atteint un enfant ; l’édition à la souris utilise les matrices inverses
des parents. Déplacement, poignée de taille et nœuds fonctionnent dans les groupes
étirés/tournés. Une régression initiale de sélection des formes superposées à la
racine a été trouvée par la suite SVG et corrigée : seule la sélection d’un enfant
garde la priorité au glisser, les formes racines respectent l’ordre visible.

Projet v7, lecture des versions 1 à 7. Les anciennes identités nulles/dupliquées
de formes sont migrées ; les arbres v7 invalides sont refusés. Limites vérifiées :
16 niveaux, 20 000 formes par calque, identités compatibles JavaScript, matrices
finies/inversibles et transformations cumulées bornées. Les écritures et les
transformations d’image valident ces invariants avant adoption/commit.

Six nouveaux tests du cœur couvrent rendu/pile/styles/undo, deux niveaux tournés
et étirés puis dissociés, opacité isolée et refus atomiques, transformations
d’image, migration v6 et arbres invalides, texte restant éditable après dissociation.
Le cœur complet passe : 87 tests réussis, deux corpus externes ignorés à ce jalon
(ils ont été rejoués à la livraison 1.4). TypeScript/Vite et cargo build passent.

work/phase3-groups-result.json : 15 contrôles natifs réussis, aucune erreur JS.
La dissociation de la fixture imbriquée a un écart de pixels nul ; sauvegarde,
réouverture et SVG réimporté conservent exactement le rendu. PDF est produit et
sa signature vérifiée ; aucune comparaison PDF spécifique aux nouveaux groupes
n’est annoncée. Le rapport lie les résultats aux SHA des sources et du binaire
18fdedfbc36a08b275da49747dd33a64348cd1c1ce7457437e3978b09e1585b6.
Les régressions native-smoke (12), SVG (14), plume/jalon 1.2 (12), PFL (46),
Prisme/Rhai (16) et 8BF (32) repassent sur ce même binaire de développement.

Limites explicites : grouper des formes non consécutives changerait la pile,
donc est refusé. Dissocier une opacité de groupe non neutre changerait les
recouvrements, donc est refusé avec une explication. Les groupes stylés avancés
et la distribution de ce travail restent ouverts. Aucun nouveau support de
dégradé, motif ou masque SVG n’est annoncé à ce jalon ; ces exigences restent I.2.
La livraison 1.4 est conservée intacte et ne contient pas ces ajouts de développement.

Processus de test : work/prisme-groups.pid, profil work/webview-groups, port 9225 ;
Vite reste sur 1420. Vérifier chemin et commande avant toute action sur le processus.

## Dégradés SVG éditables — développement après les groupes

paint.rs définit dégradés linéaires/radiaux, 2 à 64 points RGB avec opacité,
prolongement pad/reflect/repeat, unités locales ou du parent et matrice affine.
Fond et trait ont chacun leur peinture. Le rendu produit des définitions SVG
avec identifiants uniques. Les conversions de repère utilisent la vraie boîte
géométrique du moteur, y compris pour le texte, et compensent sa transformation
interne. Les booléens conservent la peinture du premier tracé dans le repère du parent.

PaintEditor règle les paramètres en brouillon ; seule l’application produit une
étape d’historique. Annuler les réglages restaure les paramètres du document.
L’aperçu de couleurs du panneau n’est pas l’aperçu asynchrone complet prévu en I.3.
Passer à une couleur unie supprime explicitement le dégradé correspondant.

La décomposition SVG conserve maintenant les paramètres des gradients normalisés
par usvg, puis compose transformation du chemin, placement et rotation du bloc.
Les traits deviennent toujours des surfaces, avec leur peinture. Les motifs,
groupes avec opacité/fusion, masques/découpes et images intégrées restent refusés
par cette décomposition, en conservant le bloc original.

Projet v8, lecture des versions 1 à 8. Les nouveaux champs sont facultatifs à
l’ouverture des anciens projets. Quatre tests Rust contrôlent référence SVG
indépendante avec transformations non uniformes et alpha, conversion des repères
sur forme et texte, persistance/undo/refus atomiques et peinture des booléens.
La suite complète passe : **91 réussites, deux corpus externes ignorés à ce jalon**.
TypeScript/Vite et cargo build passent. La fixture d’ancien refus de dégradé a
été remplacée par un vrai motif non pris en charge ; les nouveaux tests positifs
vérifient l’extension du contrat plutôt que de simplement retirer le refus.

work/phase3-paints-result.json : **15 contrôles natifs réussis**, aucune erreur JS.
Dégradés linéaire/radial, fond/trait, positions/opacité, répétition et matrice,
Appliquer/Annuler, undo/redo, repères, projet et export. La référence SVG
indépendante a un écart de pixels nul après décomposition ; le SVG exporté
est réimporté au rendu exact. Le PDF est produit, sans annoncer à ce jalon
une comparaison de rendu spécifique. Les empreintes des sources et du binaire
sont dans le rapport ; un problème d’attente du chargement initial du script
a été corrigé en attendant la barre de commandes avant les invocations.

Le bilan courant est dans BILAN_ACTUEL.md. AMELIORATIONS_30.md contient trente
propositions distinctes, dix par catégorie demandée ; elles sont proposées,
pas annoncées comme réalisées. Le reste du plan complet reste ouvert.

Processus de test courant : work/prisme-paints.pid, profil work/webview-paints,
port 9225 ; Vite sur 1420. Vérifier le chemin/commande avant toute action.
Les installateurs 1.4 sont conservés intacts ; les motifs puis les SVG complexes
et groupes stylés sont la prochaine étape de I.2.

## Motifs SVG éditables et correction PDF — développement après les dégradés

Le modèle ajoute des motifs distincts pour fond et trait : origine et dimensions
positives du carreau, espace du contenu, unités locales/parent, matrice affine et
1 à 200 formes vectorielles. Couleur, dégradé et motif sont exclusifs sur une cible.
Le rendu émet de vrais éléments SVG pattern avec identifiants uniques. Les
booléens et transformations d’image conservent les peintures dans le bon repère.

Le panneau Peinture SVG propose damier, rayures et pois originaux, édition des
positions/dimensions/rotation/couleur/alpha, ajout/retrait et ordre des éléments.
Les réglages restent en brouillon ; Appliquer crée une étape d’historique.
L’aperçu natif asynchrone montre un carreau avant sa matrice/répétition, au plus
128 × 128 pixels ; les réponses anciennes sont ignorées. Cet aperçu n’achève
pas les aperçus généraux de filtres ni les miniatures de calques du plan.

La décomposition normalise les unités du motif et conserve viewBox, contenu,
transformations et dégradés de ses enfants. La marche compose les transformations
relatives des groupes, y compris les wrappers de contenu objectBoundingBox.
Les cas imbriqués ou avec images/opacité de groupe/fusion/masques/découpes/effets
restent refusés sans remplacer le document ni son historique. Les paramètres
invalides et motifs récursifs sont refusés avant commit. Le projet passe en
v9 ; lecture des versions 1 à 9. Voir MOTIFS.md pour le contrat complet.

La comparaison réelle des PDF a trouvé un défaut absent des contrôles de simple
signature : svg2pdf 0.13 appliquait PageOptions.dpi à la géométrie sans transmettre
ce facteur aux matrices absolues de peintures. À 96 dpi, le radial et le carreau
étaient visiblement déplacés. La conversion physique est maintenant portée par
le viewBox SVG à 0,75 point par pixel ; le convertisseur travaille à 72 points
par pouce. Taille physique et éléments vectoriels sont conservés.
Les dégradés repeat/reflect et le rayon de foyer non nul, que ce convertisseur ne
représente pas correctement, sont refusés explicitement avant écriture, y compris
dans un motif ou un SVG intégré. Le message propose le SVG. Leur prise en charge
PDF reste une exigence ouverte, pas un support supposé.

Vérification : trois nouveaux tests Rust de motifs, un test de refus PDF ;
**95 tests réussis, deux corpus externes non rejoués à ce jalon**. TypeScript/Vite
et cargo build réussissent. Le binaire de développement vérifié a l’empreinte
SHA-256 354d846959176196563956145383f7f367f4906bb841fa9c0a9c71f1831a2086.

**182 contrôles Windows réussis** : motifs 20, dégradés 15, groupes 15, SVG 14,
plume/agencement 12, PFL 46, Prisme/Rhai 16, 8BF 32 et parcours général 12.
Les rapports sont work/phase3-patterns-result.json et les fichiers *-result.json
correspondants ; les scripts relancés séquentiellement utilisent le même binaire.
Aucune erreur JavaScript. La référence SVG indépendante de motif a un écart
prémultiplié maximal 2/255, moyen 0,05356 ; seuils fixés à 3 et 0,1.
Repères, projet et SVG exporté/réimporté de la fixture restent exacts.

Les deux PDF sont rendus par Poppler à 96 dpi et comparés au rendu natif composé
sur papier blanc (work/paint-pdf-result.json). Dégradés : moyenne RGB 0,35784,
maximum 3. Motif : moyenne 2,4794, maximum 207 sur les arêtes d’un pixel ;
1,596 % des pixels dépassent 32 niveaux. Le premier seuil brut à 1 % échouait :
les métriques brutes restent publiées, puis un contrôle séparé des zones à distance
d’une arête confirme maximum 1 et moyenne 0,0077. Les images ont aussi été vues.
Ce constat ne constitue pas une identité de pixels entre Poppler et resvg.

Les documents et preuves courantes sont rassemblés dans
outputs/Progression-motifs-et-bilan, avec bilan, plan et trente propositions.
La distribution 1.4 reste inchangée ; aucun installateur contenant ces ajouts
n’est annoncé. Prochaine étape : groupes stylés et SVG complexes, puis le reste
de I.2. Le contrôle croisé PFL et toutes les autres exigences ouvertes sont conservés.

## Groupes SVG avec styles — développement après les motifs

L’import éditable conserve les groupes portant une opacité, un mode de fusion
ou une isolation. Les groupes neutres restent dépliés. Les enfants deviennent
comme auparavant des tracés/surfaces, avec leurs peintures ; opacité d’ensemble
et recouvrements restent portés par le groupe. Une opacité ajoutée au bloc SVG
est conservée dans un groupe externe, sans la distribuer à ses surfaces.

GroupStyle ajoute 17 modes SVG validés et l’isolation explicite, avec réglages
Fusion du groupe / Isoler la fusion des enfants dans Formes. Le rendu, les
matrices, l’édition d’un enfant, ses nœuds, undo/redo et le projet gardent ces
états. Dissocier un contexte non neutre est refusé avec un message, sans perte
de style ni rastérisation. Plus-lighter reste refusé explicitement en PDF, dont
le convertisseur actuel ne gère pas ce mode ; le SVG reste exportable.

Les formes importées, décomposées, dupliquées et leurs descendants reçoivent des
identités récursives uniques. OpenSvgEditable valide l’arbre avant adoption.
Le projet écrit maintenant v10 et lit les versions 1 à 10. Le contrôle natif
relit un vrai projet v9 du jalon précédent, l’enregistre en v10 puis vérifie le
rendu exact. Les fichiers v9/v10 et leurs empreintes sont conservés dans les preuves.

Le rendu natif compose chaque calque séparément. L’export SVG isole maintenant
les fusions de groupes à l’intérieur de leur calque avant d’appliquer la fusion
du calque. Cette isolation n’est ajoutée que si un groupe a un mode non Normal.
Une première version isolant tous les calques ajoutait un arrondi alpha aux
peintures simples ; la suite de dégradés l’a détecté. Le chemin sans fusion de
groupe garde maintenant son rendu/export exact, avec une régression dédiée
contre le mélange involontaire avec un calque inférieur.

Un parcours numérique a trouvé un autre défaut : les NumberField arrondissaient
l’affichage à deux décimales puis validaient cet arrondi lors d’une simple
sortie de focus. Ils ne committent désormais que les saisies modifiées ; une
valeur vide/non finie ou sans effet retrouve l’affichage conservé. Le contrôle
natif vérifie une vraie coordonnée fractionnaire, un champ vidé, la tabulation,
le document et l’historique inchangés. P04 est réalisé en développement ; P07
reste ouvert pour les autres commandes qui committent sans changement.

Six nouveaux tests Rust couvrent les 17 modes avec/sans isolation, les
recouvrements, placement affine, persistance, descendants et identités,
refus atomiques et indépendance des calques. Le cœur passe : **101 réussites,
deux corpus externes non rejoués à ce jalon**. TypeScript/Vite et cargo build
passent. Binaire debug SHA-256 :
71aaff83b9c311fd69870aa67326b7c4c80c57b518e0659a4546a83f0b76d99a.

**201 contrôles Windows réussis** sur ce binaire avec l’interface de développement
Vite : groupes stylés 19, motifs 20, dégradés 15, groupes 15, SVG 14, plume/agencement
12, PFL 46, Prisme/Rhai 16, 8BF 32, parcours général 12. Les preuves lient les
sources et le binaire ; aucune erreur JavaScript. Les anciens refus de groupes
translucides ont été remplacés dans les fixtures négatives par des découpes ou
filtres réellement non pris en charge ; les nouveaux tests positifs vérifient
l’extension du contrat.

Référence SVG tournée avec dégradé et transparence : écart prémultiplié maximal
12/255, moyen 0,06171 ; deux pixels sur 153 600 dépassent 8 niveaux. Le premier
seuil maximal 4, adapté aux fixtures simples, échouait sur ces deux bords de
courbe ; les métriques restent publiées. Ce cas est contrôlé à 16, 0,15 et moins
de 0,01 % de pixels au-dessus de 8, avec inspection des images. Le SVG exporté
puis réimporté a un écart nul dans la fixture native.

Trois PDF sont maintenant comparés par Poppler à 96 dpi sur papier blanc.
Les résultats précédents de dégradés/motifs sont inchangés. Groupes stylés :
moyenne RGB 0,51760/255, maximum 34 sur les bords ; à distance d’un bord,
moyenne 0,43772 et maximum 3. Images vues et rapport work/paint-pdf-result.json
avec métriques brutes, seuils et empreintes. Les trois PDF conservent les
éléments vectoriels compatibles ; aucune identité de pixels PDF/resvg n’est supposée.

Les acquis, trente propositions et reste à faire sont mis à jour dans
BILAN_ACTUEL.md, AMELIORATIONS_30.md et PLAN.md. Les preuves de ce jalon sont dans
outputs/Progression-groupes-SVG-styles. Les paquets 1.4 restent figés et ne
contiennent pas ces ajouts. Les images intégrées, découpes, masques et effets SVG
éditables sont la prochaine étape de I.2 ; les autres parties et le contrôle
croisé PFL restent ouverts.


## 26 octobre 2026 — images intégrées et découpes SVG éditables

Suite de I.2, sans clôturer les masques/effets ni les autres exigences du plan.
Images PNG/JPEG/GIF/WebP conservant leurs données et SVG interne normalisé,
remplacement géométriquement stable, six interpolations et aperçu natif.
Unions, trous, découpes liées/imbriquées et repères objectBoundingBox sont
importés dans une structure éditable ; panneau en brouillon, géométrie, nœuds,
matrices, modèles rectangle/ellipse, retrait non destructif et Annuler.

Le clic respecte les découpes des parents même quand leur enfant est déjà
sélectionné. Les références image externes absentes sont nommées dans les
informations du document ; l’ouverture éditable refuse tout résultat partiel.
Le projet v11 stocke ces structures et migre un véritable projet v10 figé.
Un débordement SVG reste contenu dans son cadre avant/après décomposition.
Les groupes découpés ne sont pas dissociés en perdant leur découpe.

Six tests Rust ajoutés ; les anciennes fixtures négatives de découpes ont été
remplacées par des masques encore incompatibles. Les nouveaux cas positifs
comparent images et découpes, y compris transformations et contenu débordant.
Cœur : 107 réussites, zéro échec. Les deux corpus externes ont aussi été rejoués
avec succès. HEIC réellement décodé dans cette fixture ; RAW : aperçu seulement.
TypeScript/Vite et cargo build passent.

Binaire de développement debug SHA-256 :
0bc1590b2bc7d604a1eba4c2bfb3b79e2eb8a5ce228fcb767e3137fe66490cce.
300 contrôles Windows réussis : images/découpes 21, groupes stylés 19, motifs 20,
dégradés 15, groupes 15, SVG 14, plume/agencement 12, PFL 46, Prisme/Rhai 16,
8BF 32, parcours général 12, sélections 11, retouche 10, réglages 8, ressources 8,
effets 11, palette 11, actions/lots 9, formats 6 et exports 4. Aucun pageerror.
Le contrôle de l’algorithme de sélection et le clic réel sont distincts.

Référence native : import/décomposition et réimport SVG exporté sans écart de
pixels dans la fixture ; SVG interne : maximum 1/255. La normalisation du texte
en contours de découpe a d’abord échoué au seuil général : maximum 48/255 sur
51 pixels de bord. Ce cas garde un seuil distinct de 64 et moins de 100 pixels
au-dessus de 8 ; aplats contrôlés séparément à 3. Les autres cas gardent 16/0,15.

Quatre PDF comparés avec Poppler. Le premier essai des images révélait
l’interpolation ignorée par svg2pdf : moyenne RGB 1,07475, maximum 31 dans les
zones faiblement variables. Le PDF prépare maintenant les objets bitmap lissés
à leur taille de page à 96 px/pouce dans une copie d’export. Projet inchangé,
budget de 16 millions de pixels, vecteurs conservés. Après correction : moyenne
0,50512, maximum global 82 sur les bords, maximum 7 dans les transitions faibles,
maximum 1 dans les aplats constants. Le seuil spécifique des transitions bitmap
est 8, celui des aplats reste 3 ; les anciens seuils vectoriels sont inchangés.
L’ancien seuil global d’intérieur a échoué après correction et reste consigné ;
il ne faut pas présenter ces quatre références comme identiques pixel à pixel.
Images PDF et références inspectées. Les images encore enfermées dans un bloc
SVG non décomposé gardent les limites du convertisseur.

SVG_IMAGES_DECOUPES.md donne le contrat et les limites. BILAN_ACTUEL.md et PLAN.md
restent explicites sur les chantiers ouverts. AMELIORATIONS_30.md garde trente
propositions ouvertes : les anciennes P04 (NumberField) et P06 (images externes)
réalisées sont remplacées par le geste d’opacité interrompu et les feImage.
Sources, rapports, exemples v11, migration v10 et captures :
outputs/Progression-images-et-decoupes. Les anciens paquets 1.4 restent figés.
La prochaine action de I.2 est le support des masques et effets SVG éditables,
puis le texte avancé, la vectorisation et PFV. Les autres parties du plan restent
ouvertes, dont les cinq PFL originaux et la comparaison dans PhotoFiltre.


### Aperçu portable autonome du 26 octobre

Compilation optimisée Tauri avec interface embarquée, sans nouvel installateur.
EXE SHA-256 : ef0d7c45444e0a84084ead601d50a9d60b2f720c003f6790f1104b20f71a7792.
Le numéro interne reste 1.4.0 ; le dossier et l’archive portent explicitement
le nom Prisme-apercu-2026-10-26. Ils n’écrasent pas la distribution 1.4 figée.
Les projets de cet aperçu sont v11. SDK, notices et exemples sont joints.

299 contrôles Windows réussis sur cet EXE : mêmes vingt suites que le debug,
avec 20 contrôles images/découpes au lieu de 21. Le contrôle d’import direct du
module TypeScript exige Vite et est omis du portable ; le vrai clic sur la
partie cachée d’un enfant sélectionné reste vérifié dans l’interface compilée.
Le premier rapport utilisait par erreur le chemin du binaire debug : attribution
corrigée dans le script puis toutes les suites du portable rejouées. Le rapport
porte maintenant l’empreinte de l’EXE effectivement lancé.

Trois contrôles de ressources confirment l’interface embarquée, les deux scripts
Rhai et le PFL original chargés depuis le paquet avec les octets attendus.
Quatre PDF ont été régénérés sur ce portable puis comparés avec Poppler selon
les seuils documentés. Toutes ces comparaisons passent. Rapports et empreintes
propres dans VALIDATION_PORTABLE.json et dans Progression-images-et-decoupes/Portable.
La compilation optimisée ne prouve pas l’installation ou la mise à niveau.
La comparaison inter-hôtes PFL et les exigences ouvertes de PLAN.md restent à faire.


## 26 octobre 2026 — masques SVG peints et projet v12

Suite de I.2 : masques alpha/luminance représentés par des contenus peints
éditables, régions, matrices, groupes et intersections. Le panneau permet
peintures/dégradés, géométrie, nœuds, images/remplacement, ordre, fusion/isolation,
création d’un masque blanc, retrait et annulation. Le brouillon a un aperçu
natif asynchrone ; les anciennes réponses sont ignorées. Les saisies numériques
intermédiaires et la virgule décimale sont acceptées sans appliquer le document.

Le hit-testing des documents masqués vérifie le pixel rendu du candidat avec
ses ancêtres. Le point final d’un geste est conservé pendant la requête ; les
versions périmées sont refusées et Échap annule sans mutation. Une première
injection de retard dans le test visait l’API immuable de Tauri et n’était pas
effective : le transport fetch est désormais retardé et son déclenchement
explicitement contrôlé. Les tests rejoués vérifient aperçus et gestes retardés.

Les définitions de masques référencées éliminées par le parseur sont détectées
avant décomposition : conservation du bloc entier et message explicite même
avec des frères visibles. Les définitions inutilisées ne bloquent pas. Les
filtres du contenu restent refusés. Les anciennes fixtures négatives de masques
ont été remplacées par des filtres ; les masques sont couverts positivement.

Projet v12, lecture des versions 1 à 12. Migration d’un véritable v11 figé,
undo, sauvegarde/réouverture, duplication et identités des descendants vérifiés.
Les contenus des masques ont leur espace d’édition local et ne sont pas des
objets de sélection globale du calque. Les masques de calque bitmap restent à faire.

Sept nouveaux tests (six de masque et un d’export) : 114 réussites du cœur,
zéro échec, plus deux corpus externes rejoués. TypeScript/Vite et cargo build
passent. 326 contrôles Windows passent sur le même binaire, dont 26 de masques.
Référence indépendante de luminance : import/décomposition et réimport du SVG
exporté à écart nul. Une seconde référence alpha utilise une image transparente.
Aucun pageerror. Binaire debug SHA-256 :
14826c2dba7f4b9da84f571c39116d6b42de6692323372e4f2f7234a231b9793.

Le PDF révélait un écart de luminance sur les pixels colorés : premier contrôle
échoué à 12/255 dans des aplats constants. Les masques de luminance sont préparés
en couvertures alpha natives à 96 px/pouce dans une copie d’export. Leur région
est exprimée dans la page puis ramenée au repère du groupe ; transformations,
décalages et marges d’effets sont couverts. Test comparant le rendu préparé au
rendu original avec rotation, décalage de calque et ombre ; seuil maximal 4/255,
moyen 0,04. Le projet source reste inchangé. Les calques invisibles ne consomment
pas ce budget d’export. Les images des masques alpha gardent la préparation
d’interpolation ; budget cumulé de 16 millions de pixels, sinon refus explicite.

Six PDF passent les comparaisons Poppler. Les deux nouvelles références suivent
les critères existants des images pour les transitions bitmap : maximum 8/255,
aplats constants maximum 3. L’ancien contrôle vectoriel à 3 a échoué dans les
transitions de la couverture calculée ; il n’est pas présenté comme réussi.
Après correction, luminance : moyenne RGB 0,43436, maximum de bord 62, maximum 7
dans les transitions faibles et 2 dans les aplats constants. Alpha : moyenne
0,44886, maximum de bord 150, maximum 5 dans les transitions et 1 dans les aplats.
Les maxima bruts restent publiés. Images inspectées ; aucune identité PDF/resvg
n’est annoncée. pypdf confirme quatre courbes de Bézier du contenu et une image
de couverture séparée dans chaque nouvelle référence : la page n’est pas aplatie.
Les peintures et formes compatibles du contenu restent vectorielles ; la
couverture de luminance devient une image à résolution finie dans le PDF.

Sources et preuves : SVG_MASQUES.md et outputs/Progression-masques-SVG.
Les anciens installateurs 1.4 et l’aperçu autonome v11 restent figés ; ce jalon
n’y est pas annoncé comme inclus. PLAN.md conserve ses exigences ouvertes ; la
suite de I.2 porte sur les filtres/effets, puis le texte avancé, la vectorisation
et PFV. Les autres parties du plan, dont couleur/tuiles/GPU, interface/expérience
et pack PFL avec contrôle croisé, restent à exécuter.


### Complément : masques dans les SVG conservés en blocs

La correction PDF est étendue aux blocs SVG compatibles et aux images SVG
internes avec masques, dans une copie d’export. Leur décomposition manuelle
n’est pas nécessaire. L’isolation implicite de l’image SVG est conservée pour
éviter de mélanger son contenu avec un objet placé derrière. Les textes du
bloc sont convertis en contours dans cette copie. Les SVG non décomposables,
dont les filtres encore ouverts dans I.2, gardent les limites du convertisseur.

Le test d’export couvre maintenant les masques éditables, les blocs bruts et
les SVG imbriqués, avec fond dans le même calque, rotation/décalage et ombre.
Le premier maximum strict à 4 a trouvé 15 sur un bord après normalisation des
SVG imbriqués. Les scènes normalisées suivent le seuil de décomposition déjà
utilisé : maximum 16, moins de dix pixels au-dessus de 8, moyenne 0,04 ; les
masques déjà éditables gardent 4. Le test vérifie aussi le projet source intact.
Le cœur reste à 114 réussites et deux corpus externes ; pas de multiplication
artificielle du nombre de tests pour les variantes d’une même matrice.

Binaire final de ce jalon debug SHA-256 :
b5fdaef47b47ace21724a708ab30bdd53b320310ed69bbae83f321eb74be5a03.
327 contrôles Windows passent, dont 27 de masques. Les huit PDF comprennent
les quatre références antérieures, les deux masques éditables et les deux
exports de blocs bruts. Les quatre nouvelles références gardent quatre courbes
de Bézier et une image de couverture séparée, avec rapport pypdf propre.
Les maxima de bord restent publiés ; les critères des aplats et transitions
bitmap ne sont pas présentés comme une identité de pixels.

La revue visuelle a trouvé un débordement horizontal dans les champs du panneau.
Largeurs et grille du masque ont été bornées ; le contrôle natif vérifie
maintenant l’absence de débordement. Les suites d’interface affectées et la
compilation TypeScript ont été rejouées. Les preuves finales sont rafraîchies
sur ce même binaire dans outputs/Progression-masques-SVG. Aucun nouveau portable
ni installateur n’est annoncé : l’aperçu v11 et les paquets 1.4 restent figés.


## 26 octobre 2026 — chaînes d’effets SVG, développement v13

Après les masques, seize familles de filtres deviennent éditables : flou,
ombre, décalage, matrice couleur, transfert des quatre canaux, fusion,
composition, assemblage, couleur unie, morphologie, convolution, déplacement,
turbulence, carrelage et éclairages diffus/spéculaire. Le panneau propose
entrées et résultats nommés, paramètres, sous-éléments, régions et matrice.
Aperçu natif asynchrone en brouillon ; seules les réponses actuelles s’affichent.
L’application et le retrait sont annulables ; les enfants restent vectoriels.
Les filtres sont aussi convertibles dans le contenu d’un masque.

Les références indépendantes ont révélé des défauts de contexte et de grille :
les groupes filtrés gardent désormais leurs coordonnées originales. L’opacité,
la fusion, le masque et la découpe sont émis avec le filtre, plutôt que dans un
intermédiaire qui changeait le calcul. Le carrelage a également révélé une
annulation de translations en f32 ; les cadres importés gardent l’origine sans
recentrage puis retour approximatif. La sélection évalue les pixels des effets,
y compris après décalage hors de la géométrie initiale. Un calcul sur une source
1×1 était insuffisant ; la région complète est utilisée lorsque nécessaire.

Le projet écrit v13 et lit v1–v13. Une vraie archive v12 du jalon figé des masques
est migrée au rendu exact. Le cadre SVG et les chaînes persistent ; duplication
et import conservent des identités récursives uniques. Les entrées absentes,
futures/dupliquées, nombres invalides et régions excessives sont refusés sans
mutation du document. Les anciens tests de refus de flou passent désormais sur
feImage, qui reste effectivement non décomposable ; le flou CSS est ajouté aux
scènes acceptées, sans supprimer la vérification des refus atomiques.

Le moteur PDF repose sur usvg/resvg 0.45, l’éditeur sur 0.49. Les seuls sous-arbres
filtrés sont préparés en PNG natif à 96 pixels/pouce dans la copie d’export ; le
reste reste vectoriel. Le contrôle visuel a d’abord trouvé des effets absents :
l’ancien chargeur de SVG imbriqués supprime toutes leurs images internes.
La préparation développe désormais les ressources compatibles et transforme
le cadre en groupe ordinaire, avec isolation et découpe seulement si nécessaire.
Un cadre découpé inutilement ajoutait une seconde antialias sur son bord :
le test affine a empêché de conserver ce changement.

La première comparaison PDF des nouvelles scènes (moyenne RGB 25,09 pour la
scène d’effets, 79,88 pour le masque filtré) a ainsi été corrigée. Les courbes et
objets voisins restent de vrais tracés dans les PDF, vérifiés avec pypdf.
L’opacité du filtre est maintenant échantillonnée par le moteur natif avec sa
peinture, évitant une seconde quantification en opacité de groupe PDF.
Le test de copie affine observe au final cinq pixels d’arête au-delà de huit,
maximum 16, moyenne 0,0870, et un maximum d’un dans les aplats 3×3
(sous un seuil de deux) ; les
scènes sources ne sont jamais modifiées. Ces seuils sont explicites et ne sont
pas présentés comme une identité stricte.

Pour les deux PDF de la scène filtrée, le premier classement d’aplats 3×3
trouvait exactement deux pixels à quatre niveaux près du bord du cercle.
Le double échantillonnage filtre natif/image PDF a une portée supplémentaire :
le contrôle de ces deux scènes emploie une bande d’arête de deux pixels et
publie aussi le résultat initial sur un pixel. Le maximum des aplats 5×5 est
alors un. Le critère des transitions douces reste huit ; les huit références
antérieures et le masque filtré gardent leur bande d’un pixel et leurs seuils.

Validation finale : 121 tests du cœur, deux corpus externes PSD/HEIC/RAW rejoués,
349 contrôles sur l’application Windows, dont 22 spécifiques aux effets, et
11 comparaisons Poppler à 96 dpi. Les références d’effets ont un écart natif nul
à l’import et au réimport SVG ; le masque filtré indépendant a aussi un écart
nul. Sept inspections structurelles PDF confirment les géométries non filtrées
vectorielles et les images de filtre/couverture séparées. Voir SVG_FILTRES.md.

Le binaire debug de validation est identifié par l’empreinte dans
outputs/Progression-filtres-SVG/validation.json ; rapports, scripts, références,
captures, documents et instantané des sources y sont figés. Aucun installateur
ni ancien aperçu portable n’est remplacé. feImage et ses ressources restent
ouverts, puis texte avancé, vectorisation, PFV et les autres exigences du PLAN.
Les cases globales ne sont pas cochées sur la seule base de ces seize familles.


Une relance de validation a perdu sa cible WebView (« Target page, context or
browser has been closed »), alors que le processus Prisme restait actif. Cette
exécution interrompue n’est pas comptée comme un succès ; la régression finale
est rejouée intégralement dans une session neuve, avec son rapport et son
empreinte propres. La cause de cette interruption isolée n’est pas établie.

## 27 octobre 2026 — ressources feImage, développement v14

Le catalogue comporte maintenant dix-sept familles. Une étape feImage possède
une ressource typée contenant des formes éditables, plutôt qu’un href arbitraire
stocké dans le projet. PNG, JPEG, GIF et WebP gardent leurs données originales ;
les références vectorielles locales et SVG intégrés deviennent des contenus
autonomes avec géométrie, peintures, groupes, découpes, masques et filtres.
Les identités de cette copie restent locales à la ressource. Le SVG exporté
émet des définitions distinctes hors du filtre avec des identifiants propres.

Le panneau de source propose chargement, trois ajustements, neuf alignements,
remplacement d’image conservant la géométrie, interpolation, position, opacité,
matrice, peintures, nœuds et ordre. Les réglages et aperçus natifs restent en
brouillon jusqu’à l’application. Une ancienne réponse d’aperçu est ignorée ;
le chargement désactive Appliquer et une réponse après changement de cible
ne remplace pas la nouvelle sélection. Retrait, annulation et undo sont vérifiés.

La conversion emploie directement l’arbre typé du SVG source : une sérialisation
intermédiaire supprimait l’entrée des éclairages. Les définitions sont copiées
avant la conversion récursive pour éviter un emprunt imbriqué incompatible.
Un SVG source avec ombre débordante a révélé une découpe indue (maximum natif
201 avant correction) : son cadre éditable conserve désormais ce débordement.
Le test de non-régression reste dans le corpus du cœur.

Les références héritées entre filtres sont suivies avec une limite et un refus
des cycles. Images externes/manquantes, données corrompues et contenus éliminés
par le parseur sont diagnostiqués avant décomposition ; le bloc reste conservé.
Une définition inutilisée ne bloque pas à elle seule le document. Les ressources
vides ou excessives sont refusées sans mutation. Budgets : huit niveaux de chaînes,
profondeur de géométrie seize, mille formes et vingt mille nœuds par validation
de chaîne, en complément des limites existantes. Voir SVG_FEIMAGE.md.

Le projet écrit v14 et relit v1–v14. Une vraie archive v13 du jalon figé précédent
est migrée et réouverte au rendu exact. Duplication et import gardent des
identités principales uniques et des ressources valides. Une image dont les
dimensions annoncées contredisent les données est refusée atomiquement.

Le PDF prépare les seuls sous-arbres filtrés à 96 pixels par pouce, en conservant
le contexte du cadre SVG nécessaire à feImage. Les premières comparaisons de
la copie native trouvaient un maximum de 56 et une moyenne de 0,294 ; préserver
ce contexte permet de passer les critères du cœur, maximum 12 et moyenne 0,15.
Le document source reste identique ; les objets voisins restent vectoriels.

La vérification Poppler est distincte de celle du cœur. Pour les deux nouveaux
PDF bitmap, le premier classement des transitions avec une bande d’un pixel
trouvait six pixels au-delà de huit niveaux, maximum 12. Le calcul natif du
filtre puis l’échantillonnage image/alpha du PDF ajoutent une portée d’arête :
la bande de deux pixels donne un maximum de 7, moyenne 0,3998, sans relever les
limites 8 et 0,5. Les aplats constants 3×3 restent à 2 sous la limite 3. Le rapport
publie aussi les valeurs initiales. Les douze autres scènes gardent leur
classement des transitions ; les anciennes références gardent leurs seuils.
La moyenne RGB globale des deux nouveaux PDF bitmap est 0,6978 ; les arêtes
peuvent atteindre 209, avec 0,2481 % des pixels au-delà de 32. La référence
vectorielle feImage passe à un pixel, maximum 6 dans les transitions douces et
1 dans les aplats. Le PDF reste un rendu à résolution finie, sans promesse
d’identité stricte ni de conservation vectorielle des effets échantillonnés.

Validation finale : **127 tests du cœur et deux corpus externes** PSD/HEIC/RAW,
**370 contrôles Windows**, dont **21 nouveaux**, **14 comparaisons PDF** et
**10 inspections structurelles PDF**. Aucune erreur JavaScript dans les rapports.
Les deux références indépendantes feImage bitmap et vectorielle ont un écart
natif nul à la conversion ; le SVG exporté testé se réimporte avec un écart nul.
La compilation de l’interface est validée. Les rapports, scripts, projets,
captures, références, documents et instantané des sources sont figés dans
outputs/Progression-feImage-SVG/validation.json. Le binaire de validation porte
l’empreinte SHA-256
`3da7af06ae7eba453c30b7eed68185a4ac98dc9d5c8dc8f8f0c3474b88d27287`.

Les anciens paquets 1.4 et l’aperçu v11 restent inchangés. Ce jalon ne clôture
pas toute la phase 3 ni le PLAN : texte sur tracé, polices variables, styles,
vectorisation, PFV, distribution et autres exigences restent ouverts.

## 27 octobre 2026 — texte sur tracé, développement v15

Le texte peut suivre un guide lié du même calque : tracé, ligne, ellipse ou
rectangle sans arrondi, avec choix du contour. Le panneau propose départ en
pixels ou pourcentage, alignement début/centre/fin, écart et sens de parcours.
Contenu, police, corps, gras, italique et peintures restent éditables. L’aperçu
natif est asynchrone et en brouillon ; les réponses périmées sont ignorées.
Appliquer ajoute une étape annulable. Voir TEXTE_SUR_TRACE.md pour l’utilisation.

Les cadres des groupes sont pris en compte pour relier un texte et un guide
de parents différents. Modifier le guide ou ses nœuds recalcule les lettres et
leur boîte native. Grouper/dissocier garde le lien ; duplication et import
réattribuent ses identités avec celles des formes. Détacher conserve une copie
de la courbe pour permettre un déplacement indépendant. Le retour au texte
droit et la conversion explicite en contours sont distincts et annulables.

Une suppression ou un remplacement du guide qui casserait un lien est refusé.
L’exécution prépare désormais document et historique ensemble : un refus après
modification géométrique ne laisse pas une étape d’historique engagée. Les
masques et ressources de filtre demandent de détacher les liens avant intégration.
Les actions n’enregistrent pas la création d’un texte lié à des identités non
stables ; le texte détaché garde le contrat existant de création de forme.

Le projet écrit v15 et relit v1–v15. Une véritable archive v14 du jalon feImage
est migrée et réouverte au rendu exact. Le SVG et son optimisation conservent
textPath et un guide propre à l’export ; la référence Windows se réimporte
avec un écart nul. La décomposition SVG garde son contrat explicite de texte
en contours ; le projet est nécessaire pour conserver les liens éditables.

Le PDF prépare les glyphes courbes en contours vectoriels avec le moteur natif,
dans une copie du document. Les polices utilisent des quadratiques ; les tracés
éditables utilisent des cubiques. Leur lissage peut donc différer aux arêtes.
La première vérification du cœur observait maximum 41, moyenne 0,00936 et
106 pixels au-delà de huit sur 119 600 ; le critère propre aux contours est
48, 0,02 et 0,1 % de surface. Une première version du groupement observait
maximum 14, moyenne 0,0001254, deux pixels au-delà de huit ; la conservation
du cadre propre au texte donne un écart nul dans la référence finale simple.
Les comparaisons de texte éditable gardent leurs critères séparés (12 et 0,15).

À 34 px, la conversion Windows observe maximum 52, moyenne 0,01237 et 339
pixels au-delà de huit sur 268 800 ; limites 64, 0,03 et 0,15 %. Après la
normalisation supplémentaire de la décomposition SVG : maximum 52, moyenne
0,02558 et 591 pixels, soit 0,2199 %, sous des limites 64, 0,03 et 0,25 %.
Ces premiers dépassements sont consignés, avec des critères distincts selon
la transformation. Ils ne sont pas annoncés comme une identité de pixels.

Un contrôle supplémentaire a identifié un défaut de glisser sur le texte lié :
la branche de déplacement pouvait retomber sur le déplacement du calque.
Le glisser est maintenant ignoré pour cette forme liée et ne dessine plus de
cadre déplacé en aperçu. Un essai de souris maintenue puis relâchée vérifie
cadre, pixels, position du calque et historique inchangés. La régression
complète est rejouée après cette correction sur le binaire final.

Un dernier essai sur groupe redimensionné et tourné a trouvé une vraie perte de
transformation des lettres à la dissociation : maximum 219, moyenne 1,7724 et
2 698 pixels au-delà de huit. Le repère propre du texte était supprimé en
recalculant son guide. Il est maintenant conservé et le guide est résolu dans
son inverse ; cela préserve la taille et l’affinité des lettres. Le neuvième test
du cœur garde cette régression et obtient un écart nul. Le vingt-sixième
contrôle Windows mesure maximum 13, moyenne 0,00006417 et trois pixels au-delà
de huit sur 268 800, sous des limites 24, 0,01 et 0,05 % de surface. Document,
historique, duplication et migration sont rejoués après cette correction.

Validation finale : **136 tests du cœur**, deux corpus externes PSD/HEIC/RAW,
**396 contrôles Windows**, dont **26 nouveaux**, **15 comparaisons PDF** et
**11 inspections structurelles PDF**. Les références du texte lié, du groupement
Windows et des SVG exportés ont un écart nul. Le PDF de texte a une moyenne RGB
de 0,11590, maximum 119 aux arêtes et 0,08371 % des pixels au-delà de 32.
Les aplats au-delà d’une bande d’arête d’un pixel ont un écart nul, sous les
anciens seuils vectoriels. Ses 236 courbes et l’absence d’image XObject sont
vérifiées : les lettres de cette scène restent vectorielles. Les quatorze autres
comparaisons PDF gardent leurs seuils et leurs classements précédents.

Rapports, scripts, références, captures, archives, documents et sources v15
sont figés dans outputs/Progression-texte-sur-trace/validation.json. Binaire
debug final SHA-256 :
`4987301489802b82f02f642931fd4bd2efb9794ecfe88a22e1c8a1c1a79c65ba`.
Les paquets 1.4, l’aperçu v11 et les jalons précédents ne sont pas remplacés.
La prochaine étape porte sur les polices variables, les styles de texte et
le remplacement explicite d’une police absente ; la distribution et les autres
exigences du PLAN restent ouvertes.

## 27 octobre 2026 — Polices variables et styles réutilisables, projet v16

Le catalogue système est partagé avec le moteur natif. Les axes fvar réellement
présents, leurs plages et leurs valeurs sont exposés au texte droit et courbe,
avec brouillon, aperçu asynchrone, réponses périmées ignorées, application en une
étape et annulation. Les valeurs explicites priment sur les automatismes ; opsz
peut suivre le corps ou utiliser le défaut de la police. Une famille absente est
signalée avec son repli sans réécrire le nom demandé. Le remplacement explicite
nettoie les axes de l’ancienne famille et reste annulable. Les axes étrangers sont
préservés dans le projet avec diagnostic, plutôt que réécrits à la lecture.

Les styles copient la typographie et les peintures sans contenu, position, repère
ni lien de tracé. Application simple/multiple atomique, sauvegarde dans le projet
et l’historique, mise à jour/suppression et bibliothèques JSON version 1 sont
réalisées. Les collisions sont renommées ; limites de 128 styles et 4 Mio par
bibliothèque. Les modèles sont des copies, sans propagation aux textes existants.
Le chemin PFL conserve la bibliothèque lorsqu’il remplace les pixels du document.

Les tests ont révélé que le sérialiseur SVG optimisé d’usvg 0.49 oublie les
propriétés CSS de variation, même si son moteur les rend correctement. L’export
conserve le SVG source fidèle dans ce cas ; réduction et précision ne sont pas
garanties pour ces documents. Le convertisseur PDF plus ancien reçoit des contours
natifs dans une copie du document. Le source conserve son texte éditable et ses axes.
Un débordement de quatre pixels du curseur est corrigé par ses marges. La sélection
native est aussi étendue aux textes droits : les glyphes variables peuvent déborder
du cadre nominal, et un clic réel au-delà de ce cadre est maintenant vérifié.

Sept nouveaux tests du cœur utilisent une police originale réservée aux tests,
avec neuf maîtres et deux axes. FontTools calcule séparément contours et avances.
La vraie Segoe UI Variable installée possède wght 300/400/700 et opsz 5/10,5/36 ;
son empreinte et la méthode sont consignées sans distribuer la police Windows.
Les deux références droites et le SVG textPath indépendant ont un écart nul.
Les contrôles d’interface attendent la fin du délai de regroupement des aperçus
avant lecture ; une réponse plus ancienne ne doit jamais gagner.

Validation finale : 143 tests du cœur, deux corpus externes PSD/HEIC/RAW,
419 contrôles Windows dont 23 nouveaux, aucune erreur JavaScript, 17 comparaisons
PDF et 13 inspections structurelles. Les anciens critères PDF sont conservés.
Les nouveaux PDF droit/courbe ont des moyennes RGB de 0,25899/0,12434 et des maxima
d’arête de 96/104 ; les aplats au-delà de la bande d’arête d’un pixel sont identiques.
Ils n’ont aucune image XObject ; le second comprend 169 courbes de Bézier.
Les projets v16 relisent v1–v16 ; une véritable archive v15 est migrée au rendu exact.
La compilation de l’interface et du binaire natif réussit.

Preuves, scripts, captures, projets, bibliothèques et sources v16 sont figés dans
outputs/Progression-polices-styles/validation.json. Binaire debug SHA-256 :
`daa41860723da263cc50c11115dcb01fb07abf7dbed4b6bb1c2b5a201338d232`.
Les installateurs 1.4, l’aperçu portable v11 et les jalons précédents sont conservés.
Les polices restent requises sur le poste cible ; les styles liés en direct et
la mise en page multiligne ne font pas partie de ce contrat. Voir POLICES_STYLES.md.
Prochaine étape : vectorisation bitmap monochrome/couleur en tracés éditables,
puis PFV ; distribution et reste du PLAN encore ouverts.

## 27 octobre 2026 — Vectorisation bitmap en tracés éditables

Calque > Vectoriser le bitmap ouvre un dialogue monochrome/couleur avec seuil,
inversion, couleur de sortie, palette RGBA 2–32 entrées, seuil de transparence,
résolution 32–1024, petites zones, simplification et arrondi. Les pixels
échantillonnés et le résultat sont comparés dans le dialogue ; les demandes sont
regroupées et leurs réponses périmées ignorées. Annuler/Échap n’engage aucun
historique et les raccourcis du document sont suspendus dans ce dialogue.

Un identifiant de révision du document est exposé au dialogue. Aperçu et
application refusent les anciennes révisions. Le résultat est recalculé dans
la transaction avant installation : nouveau calque, bitmap source conservé,
option pour le masquer, reprise de position/opacité/fusion/effets et une seule
étape annulable. Le masque de sélection peut multiplier l’alpha en coordonnées
du document, avec calque décalé. En mode indexé, un passage RVB est requis ;
le GIF doit être extrait en image fixe. L’enregistrement en actions/lots n’est
pas proposé pour cette commande à révision et cible explicites.

Le calcul construit les arêtes des régions par entrée de palette, suit les
contours en préservant les contacts diagonaux et les trous, retire les points
collinéaires, simplifie sans récursion puis transforme les angles en cubiques.
Les petites îles sont filtrées avant le graphe ; les petits trous peuvent être
remplis. Grille ≤1 048 576 pixels, 200 000 arêtes, 1 000 contours, 20 000 nœuds
et budget de subdivision borné. Tout dépassement refuse la totalité du résultat.
Les chemins obtenus sont des formes Prisme ordinaires, avec nœuds éditables.
Le format de projet reste donc v16, lecture v1–v16 ; aucune nouvelle version
n’est annoncée pour des structures déjà présentes.

Huit nouveaux tests du cœur couvrent les 511 motifs binaires 3 × 3 non vides,
les références RGBA indépendantes, seuil/inversion/alpha, petites îles et trous,
bruit, sélection douce avec décalage, résolution réduite, paramètres invalides,
révisions périmées, refus atomiques, propriétés, historique et exports. Tous les
511 motifs conservent exactement leurs pixels. Le lissage a un contrôle distinct
avec une référence quadratique indépendante : première mesure max 48 et moyenne
0,13845 à 24 px ; seuils 64/0,2, au plus 16 pixels d’écart alpha >8 et aplats
exacts hors bande d’un pixel. L’opacité de calque 0,7 peut différer d’une unité
d’alpha entre composition bitmap et SVG (moyenne 0,07324) ; ce seuil distinct
n’est pas présenté comme une identité des pixels. Les autres tests exacts le restent.

Validation finale : 151 tests du cœur et les deux corpus externes PSD/HEIC/RAW,
443 contrôles Windows dont 24 nouveaux, aucune erreur JavaScript, 19 comparaisons
PDF et 15 inspections structurelles. Les sources pixel de Python/Pillow sont
indépendantes du traceur. L’aperçu couleur, le résultat appliqué, le SVG et son
optimisation ont un écart nul dans la référence avec trous et semi-transparence.
Le clic réel sur les chemins, l’insertion d’un nœud, undo/redo, les petites
fenêtres, la conservation de la source et une vraie archive v16 sont vérifiés.
Les deux nouveaux PDF n’ont aucune image XObject ; le second contient 65 courbes.
Moyennes RGB 0,21431 et 0,25686 ; maxima d’arête 96 et 79 ; aplats identiques
au-delà de la bande d’un pixel, sous les mêmes anciens critères vectoriels.

Sources, scripts, bitmaps, projets, captures et rapports sont figés dans
outputs/Progression-vectorisation/validation.json. Binaire debug SHA-256 :
`b102fcaea3d0d80bd73ff5025faf87ed7059dacf2e5eed2fd1caa85b65ab6892`.
La compilation de l’interface et du natif réussit. Les distributions existantes
et les preuves des jalons antérieurs sont conservées.

Voir VECTORISATION.md pour les approximations et limites. Le plan compte encore
14 lots de niveau I.1–IV.3 ouverts ; plusieurs sont partiellement réalisés.
La vectorisation ne ferme pas I.2, qui inclut PFV et des critères de couverture
et de distribution. Prochaine fonction : import PFV sur références indépendantes.

## Jalon expérience et PFV — 27 octobre 2026

Les quatre lots III.1 à III.4 sont réalisés. Voir EXPERIENCE.md pour leur usage,
contrats de verrous, bornes, préférences et récupération ; PFV.md pour le complément
vectoriel. Le plan conserve dix grands lots ouverts. Les preuves et sources v17
sont figées dans outputs/Progression-experience-PFV ; les anciens jalons restent figés.
La nouvelle construction autonome est fournie séparément des anciens installateurs.

Validation du développement : 162 tests du cœur, deux corpus externes, 477 contrôles
dans 28 suites Windows, puis cinq contrôles avec arrêt/redémarrage du processus,
19 comparaisons PDF et 15 inspections structurelles. La sauvegarde périodique réelle
a été observée après 59,8 s ; 597 battements d'une minuterie de l'interface ont été
traités pendant cette attente. Le rapport garde la mesure exacte. Les performances
mesurées sur 31 calques ne constituent pas une garantie sur toute image ou machine.
