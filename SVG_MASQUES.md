# Masques SVG peints — développement du 26 octobre 2026

Ce document conserve le jalon des masques v12. Le développement v13 et les filtres sont décrits dans SVG_FILTRES.md.
Suite de I.2. Ces ajouts sont dans les sources de développement ; l’aperçu
portable déjà figé reste au projet v11. Les exigences de PLAN.md restent ouvertes.

## Manipulation

Ouvrir un SVG éditable ou décomposer un bloc conserve maintenant ses masques
alpha/luminance sous forme de contenus peints éditables. Sélectionner le groupe,
puis ouvrir Masque vectoriel. Les unités du SVG sont normalisées en coordonnées
locales et matrices ; les repères objectBoundingBox conservent leur placement.

Luminance et transparence : le noir cache, le blanc révèle, les couleurs et
l’opacité modulent la couverture. Transparence seule : la couleur ne change pas
la couverture. Les intersections liées et les masques des groupes imbriqués se
choisissent par Étape. Les contenus gardent leur ordre, peintures et transformations.

Le panneau permet position/taille/rotation/opacité, fonds/traits/dégradés/motifs,
nœuds et tangentes, images et remplacement, ordre de superposition, région et
matrices. La fusion et l’isolation des groupes du masque restent modifiables.
L’aperçu montre sa couverture, calculée par le moteur natif. Les réglages restent
en brouillon jusqu’à Appliquer le masque ; appliquer une peinture met seulement
à jour ce brouillon. Retirer le masque révèle le contenu du groupe et s’annule.
Vider un masque cache le groupe sans supprimer ses formes ; Annuler le restaure.

Les champs du panneau acceptent les saisies intermédiaires et la virgule
décimale. Une saisie non finie ou invalide est restaurée à la sortie du champ.
La sélection utilise le rendu d’un pixel du candidat avec ses ancêtres : les
zones cachées laissent atteindre l’objet dessous, y compris quand un enfant
était déjà sélectionné. Les déplacements rapides conservent leur point de fin
pendant ce calcul ; Échap annule le geste. Une réponse périmée n’agit pas sur un
autre état du document. Les poignées et la navigation permettent toujours de
modifier explicitement un contenu hors de sa couverture.

## Persistance et limites

Projet v12, lecture des versions 1 à 12. Les masques et leurs contenus sont
stockés dans le projet ; les images restent intégrées. La distribution 1.4 et
l’aperçu autonome v11 ne lisent pas ce nouveau format. Conserver les originaux.
Huit niveaux de masques, seize de groupes, 1 000 formes et 20 000 nœuds par masque,
matrices inversibles et coordonnées bornées. Les plafonds des images, du JSON,
du projet et de l’historique restent ceux de SVG_IMAGES_DECOUPES.md/README.md.

Un masque vide ou invalide peut conduire le parseur à éliminer une branche du
SVG. Si une définition référencée disparaît ainsi, la décomposition entière est
refusée avec son nom, même s’il reste un frère visible. Un masque inutilisé ne
bloque pas l’import. Le bloc original et le document précédent sont conservés.
Ce document décrit le jalon v12. Depuis le jalon v13, seize familles de filtres
dans un masque sont éditables ; feImage reste en bloc. Voir SVG_FILTRES.md. L’édition graphique
directe des nœuds du masque, les masques de calque bitmap et la récupération des
structures éliminées par le parseur restent à réaliser dans leurs périmètres.

Le SVG exporté conserve les masques, leurs contenus et les vecteurs. Le PDF utilise
une couverture alpha calculée pour les masques de luminance, dont les coefficients
RGB diffèrent selon le convertisseur/lecteur. Cette couverture est une image à
96 pixels/pouce ; le contenu du groupe reste vectoriel. Les images lissées des
masques alpha sont pré-échantillonnées comme les autres images. Les sources du
projet restent inchangées. Budget cumulé : 16 millions de pixels pour préparer
images et couvertures ; un dépassement refuse le PDF et propose le SVG.
Les couvertures tiennent compte des transformations, décalages de calque et de
la marge des effets. L’agrandissement du PDF révèle cette résolution finie. Les SVG compatibles conservés en blocs et les images SVG internes contenant des masques sont normalisés dans la copie d’export, sans décomposition manuelle du document. Leur isolation est conservée et leur texte est converti en contours dans cette copie. Depuis le jalon v13, les chaînes de filtres compatibles sont préparées par le moteur natif pour le PDF. Une structure de ressources non décomposable est refusée explicitement à l’export PDF ; le SVG conserve le bloc. Voir SVG_FILTRES.md.

## Preuves

Six tests spécifiques de masque et un test d’export portent le cœur à 114
réussites. Les deux corpus externes PSD/HEIC/RAW passent aussi. Les tests couvrent
les modes, peintures, images, intersections, repères, transformations, masques
imbriqués, undo, persistance, normalisation sans perte silencieuse et sélection.
La préparation PDF est comparée au rendu natif avec rotation, décalage et ombre ;
seuils maximal 4/255 et moyen 0,04 pour la couverture ; la normalisation des SVG imbriqués accepte 16/255 sur moins de dix pixels au-dessus de 8, avec la même moyenne. Le projet original est contrôlé inchangé.

27 contrôles ciblés dans la fenêtre Windows vérifient le panneau, peintures,
nœuds, région, ordre, remplacement d’image, masque vidé/blanc, annulation,
sélection et gestes rapides, sauvegarde v12 et migration d’un véritable v11,
duplication, erreurs, refus de filtres et exports. Un premier dispositif de test
ne pouvait pas remplacer l’API immuable de Tauri : les retards ont été déplacés
sur le transport et leur déclenchement est maintenant contrôlé explicitement.
Le test vérifie qu’une ancienne réponse alpha n’écrase pas l’aperçu de luminance.

Référence luminance : import/décomposition et réimport du SVG exporté à écart nul.
Les références PDF alpha et luminance sont rendues avec Poppler,
avec les quatre références précédentes et deux exports depuis des blocs SVG. Le premier PDF de luminance a échoué à
12/255 dans des aplats constants ; la couverture alpha corrige cet écart à 2.
Les transitions de ces couvertures bitmap suivent les seuils déjà retenus pour
les images (8/255 dans les zones faiblement variables, 3 dans les aplats constants).
Les métriques brutes, y compris les bords où les moteurs diffèrent davantage,
incluent un maximum global de 62/255 en luminance et 150/255 en alpha, avec des
moyennes respectives de 0,43436 et 0,44886 ; les aplats constants restent à 2 et 1.
Ces maxima de bord ne sont pas une identité de pixels entre les deux moteurs.
Les métriques et les seuils
restent dans le rapport. Les seuils des anciennes références vectorielles restent
inchangés. La structure PDF contient quatre courbes de Bézier du contenu et une
image séparée de couverture dans chacune des quatre nouvelles références ; le document entier
n’est pas aplati. Cette fixture ne prouve pas toutes les variantes SVG possibles.

Les rapports, captures, références indépendantes et exemples v12 sont réunis dans
outputs/Progression-masques-SVG. Ce jalon ne constitue pas un nouvel installateur
ni une clôture des exigences de PLAN.md.
