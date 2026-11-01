# Calques, gestes et continuité — lots III.1 à III.4

Jalon du 27 octobre 2026. Les quatre lots de la partie III sont réalisés et
vérifiés dans le périmètre ci-dessous. Le reste de PLAN.md conserve ses exigences.
Projet écrit en **v17**, lecture des versions **1 à 17**. Une vraie archive v16
du jalon figé de vectorisation est migrée sans changement visuel ; ses nouveaux
verrous sont désactivés par défaut.

## III.1 — Miniatures réelles

La liste affiche le contenu isolé de chaque calque, bitmap ou vectoriel, texte,
groupes, transparence, position et effets compris. Un calque masqué conserve son
aperçu et son bouton de visibilité ; le calque actif et la sélection multiple
ont des repères distincts. Le mode de fusion est neutralisé dans cette vue isolée,
puisqu'il dépendrait des autres calques ; l'opacité est conservée.

La miniature représente le canevas entier, centré sur un damier. Les tailles
32/48/64/96 px sont mémorisées ; la densité d'écran détermine la résolution native,
bornée à 256 px. Le calcul est asynchrone, limité à deux rendus simultanés, aux
lignes visibles et à une petite marge de défilement. Les travaux en attente devenus
inutiles sont ignorés. Le cache contient au plus 256 résultats, identifiés par
révision, calque, taille et densité. Une ancienne miniature disparaît immédiatement
quand sa clé change ; une erreur laisse un état explicite, jamais une autre image.
Les révisions sont uniques entre les documents d'un même processus.

Le calcul n'alloue pas de bitmap vide de la taille du document. Les images et
effets existants gardent toutefois leurs propres coûts de rendu : ce jalon ne
remplace pas les futurs traitements tuilés/GPU de I.3.

Ce lot n'ajoute pas de masque de calque bitmap rééditable. Un masque PSD appliqué
aux pixels ne devient pas éditable. Les masques vectoriels existants restent
attachés aux formes et disposent de leur panneau et aperçu dédié ; la future
fonction F22 garde ses exigences de masque de calque lié/délié et de miniature propre.

## III.2 — Manipulation des calques

- Glisser-déposer avec indicateur de destination ; boutons Monter/Descendre.
- Double-clic ou F2 pour renommer en ligne, Entrée pour valider, Échap pour annuler.
- Ctrl pour ajouter/retirer, Maj pour une plage ; duplication, suppression et
  affichage/masquage groupés en une seule étape d'historique.
- Menu accessible par clic droit ou « Opérations sur les calques » : renommage,
  ordre, duplication, suppression, visibilité, fusion avec le dessous, verrous,
  isolation et effets. Les commandes groupées concernent la sélection ; ordre,
  renommage, fusion et effets concernent le calque actif.
- Recherche par nom, isolation temporaire et restauration des visibilités antérieures.

Les verrous sont enregistrés dans le projet et contrôlés par le moteur :

| Verrou | Contrat |
|---|---|
| Position | Refuse déplacement/réordonnancement et transformations de géométrie. |
| Pixels | Refuse peinture, retouche, filtres, effets, remplacement du contenu, plugins et édition de géométrie/style vectoriels. |
| Alpha | Bitmap seulement : conserve la couverture alpha et les couleurs des pixels totalement transparents lors des retouches. Un changement de dimensions incompatible est refusé. |

Supprimer un calque verrouillé et fusionner/aplatir des calques concernés par un
verrou sont refusés. Certaines opérations globales demandent de déverrouiller les
calques concernés plutôt que d'interpréter implicitement une exception.
La visibilité, le nom, l'export, la sélection et undo restent utilisables.
Le retour de PFL est aussi contrôlé sur les calques réellement touchés.
Une erreur ne produit aucune modification partielle. La suppression du dernier
calque est refusée ; le total reste borné à 256 calques.

L'isolation est temporaire dans l'interface. Si la structure change entre
l'isolation et sa restauration, l'ancien ensemble de visibilités incompatible
est refusé ; undo permet de revenir à la structure antérieure.
Les opérations multiples sont réservées aux documents RVB fixes.

## III.3 — Gestes et réglages

Le zoom reste ancré sous le pointeur ; Espace déplace temporairement la vue.
Adapter, Ctrl 0 et Ctrl 1 donnent accès à l'ajustement et à 100 %. Le cercle du
pinceau suit sa taille. Les valeurs numériques, bornes et unités restent visibles.
Taille, opacité, tolérance et largeur de trait sont mémorisées par outil dans les
préférences locales ; une commande les réinitialise.

Ctrl K ouvre la recherche avec raccourcis visibles, favoris et commandes récentes,
conservés entre sessions. Entrée exécute le premier résultat disponible ; Tab reste
dans le dialogue, Échap le ferme. Les couleurs récentes, l'échange premier/arrière-plan
et les largeurs des panneaux sont accessibles dans « Couleurs récentes et atelier ».
Les panneaux peuvent être masqués depuis la recherche et réinitialisés. Les outils
restent aux emplacements de l'atelier PhotoFiltre existant.

Les dialogues texte et transparence proposent la pipette système lorsque WebView2
expose EyeDropper ; le bouton est désactivé sinon. Cette dépendance est affichée
par son état, sans prétendre fournir une pipette système sur tous les environnements.
La pipette habituelle du document reste disponible dans la palette d'outils.

Les réglages bitmap ont un aperçu natif avant/après sur une copie, sans modifier
les pixels ni l'historique. Le brouillon est temporisé de 180 ms ; un seul calcul
d'aperçu de réglage est lancé à la fois, les demandes remplacées ne sont pas exécutées.
Réinitialiser retrouve le brouillon initial ; Appliquer exécute les valeurs courantes
en une étape, Annuler conserve le document. Les aperçus spécialisés des ressources,
effets, polices et formes restent présents.

« Annuler cet aperçu » retire la demande et ignore son résultat tardif. Un calcul
CPU déjà démarré peut finir sur sa copie ; cette commande ne prétend pas l'interrompre
au milieu de son algorithme. Les hôtes de plugins conservent leurs contrats propres,
dialogues et délais : aucun aperçu universel de plugin n'est annoncé. Les lots
gardent leur progression et annulation entre étapes/fichiers. Les calculs dont la
durée n'est pas mesurable affichent un état de travail ; les onglets sont désactivés
pendant l'application d'une commande, pour maintenir sa cible.

## III.4 — Plusieurs documents et récupération

Ouvrir et Nouveau créent des onglets, sans écraser l'image précédente. Chaque onglet
garde son éditeur, son historique, ses sélections, son calque/sa forme actifs,
son zoom et son déplacement. Les plugins/actions visent l'onglet actif ; les lots
continuent à traiter des copies séparées. Huit documents maximum ; l'ouverture
refuse un ensemble qui dépasserait 768 Mio d'historique au moment de ce contrôle.
Chaque éditeur conserve aussi son budget propre ; ce n'est pas un gestionnaire
global de mémoire tuilée.

Les vingt fichiers récents comportent une vignette et leur chemin complet. Un
fichier manquant est marqué et ne s'ouvre pas silencieusement. La fermeture d'un
onglet modifié propose Enregistrer/Abandonner/Annuler. La fermeture de la fenêtre
traite tous les documents et respecte l'annulation.

Toutes les **60 secondes**, les documents modifiés produisent des projets de
récupération. L'écriture du projet et du manifeste utilise un fichier temporaire,
sa synchronisation disque et un renommage atomique. La compression utilise une
photographie cohérente ; le verrou de peinture est libéré pendant l'écriture.
Une modification intervenue ensuite attend le passage périodique suivant.
Les écritures de récupération sont bornées à 512 Mio par passage et 32 entrées.
Une erreur est signalée ; elle ne marque pas le document comme enregistré.

Après interruption, « Fichiers récents et récupération » propose les copies.
Restaurer ouvre un nouvel onglet modifié, sans chemin de sauvegarde associé.
L'utilisateur choisit où l'enregistrer. Une récupération de la session courante
devenue propre est nettoyée après sauvegarde normale. Les récupérations d'une
session interrompue sont conservées jusqu'à leur retrait explicite, même si une
nouvelle session effectue sa propre sauvegarde périodique. Elles ne restaurent
pas l'historique complet d'avant l'interruption. Une interruption avant le premier
passage peut perdre les modifications de cette dernière minute.

Stockage : dossier `workspace` des données de l'application. Les essais utilisent
un dossier QA séparé via PRISME_WORKSPACE_DIR et n'affectent pas les fichiers de
récupération de l'utilisateur.

## Preuves

Le dossier figé `outputs/Progression-experience-PFV` contient les sources v17,
rapports, fixtures originales, projets, captures et empreintes. Sept nouveaux
tests du cœur couvrent les miniatures indépendantes, verrous, groupes d'opérations,
historiques, récupération après redémarrage et limites des onglets ; quatre
autres couvrent PFV. Total : 162 tests et deux corpus externes.

Les 19 contrôles du parcours principal, 15 contrôles de cas limites et cinq
contrôles avec arrêt/redémarrage du processus s'ajoutent à la régression existante.
Le test de 31 calques mesure le délai d'affichage et le nombre initial de miniatures
dans son rapport ; ce chiffre décrit cette machine, sans promesse universelle.
La haute densité à 200 %, la migration v16, la fermeture réelle de fenêtre annulée,
la conservation de vue et les réponses d'aperçu retardées sont exercées.

La validation ne clôt ni la couleur 16 bits/ICC, ni le GPU, ni les autres familles
de fonctions du plan. Le nouvel aperçu autonome est une version de développement ;
les anciens installateurs restent des jalons distincts.
