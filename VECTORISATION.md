# Vectorisation bitmap — développement après les polices et styles

## Parcours

Choisir un calque bitmap puis **Calque > Vectoriser le bitmap…**.
Le dialogue compare les pixels échantillonnés et les tracés proposés, avant
toute modification du document. Les réglages restent en brouillon et les
réponses d’aperçu périmées sont ignorées. Annuler ou Échap ferme sans appliquer.
Les raccourcis d’édition du document sont suspendus dans ce dialogue.

**Créer le calque vectoriel** ajoute un calque au-dessus du bitmap. Celui-ci
reste dans le document, masqué par défaut ; une option permet de le laisser
visible. L’application est une étape annulable. Position, opacité, fusion et
effets de calque sont repris sur le nouveau calque ; l’aperçu montre les pixels
du contenu, sans ces propriétés de composition. Les nœuds sont modifiables avec
les outils de tracés existants : ce ne sont pas des images cachées dans un SVG.

Un identifiant de révision protège l’aperçu et l’application. Si le document
a changé, une ancienne demande est refusée ; aucun résultat calculé sur une
ancienne image n’est installé. L’application recalcule les tracés à partir de
la même révision et des mêmes réglages, dans la transaction du document.

## Réglages

- **Monochrome** : seuil de luminosité 0–255, inversion et couleur de sortie.
  La luminance utilise 0,2126 R + 0,7152 V + 0,0722 B. Sans inversion, seuls
  les pixels strictement plus sombres que le seuil sont retenus. L’inversion
  prend son complément, y compris les pixels égaux au seuil. L’alpha est regroupé
  sur quinze niveaux non nuls, espacés de 17 ; les pixels transparents sont exclus.
- **Couleur** : palette RGBA de 2–32 entrées, transparence comprise, obtenue
  par le quantificateur déterministe existant sans tramage. Les couleurs réellement
  utilisées sont affichées. Il ne s’agit pas d’un changement du mode du document.
- **Seuil de transparence** : ignorer les pixels dont l’alpha est inférieur
  à la valeur 1–255. Les autres gardent une opacité quantifiée selon le mode.
- **Résolution maximale** : grille de calcul de 32–1024 pixels sur le grand
  côté, sans agrandissement. L’échantillonnage au voisin le plus proche ne crée
  pas de couleur depuis les pixels transparents. La sortie retrouve les dimensions
  d’origine. Les détails plus fins que cette grille peuvent être perdus.
- **Simplification** : écart 0–4 pixels dans les dimensions d’origine pour
  réduire les sommets des polygones. Zéro ne retire que les sommets collinéaires.
- **Arrondi des angles** : 0–1. Les angles deviennent des portions cubiques
  avec poignées éditables. Ce réglage modifie volontairement la silhouette.
- **Petites zones** : surface minimale 0–4096 px² dans les dimensions d’origine.
  Les petites îles sont retirées avant la construction des contours. Les petits
  trous peuvent être remplis. Zéro garde ces détails.
- **Limiter à la sélection** : multiplication de l’alpha par le masque,
  y compris sa progressivité, avec prise en compte du décalage du calque.
  Masquer ensuite le bitmap masque aussi ses pixels hors de cette sélection.

## Contours, budgets et limites

Chaque entrée de palette utilisée produit un tracé fermé composé en pair-impair,
avec ses îles et ses trous. Les contacts en diagonale conservent des régions
à quatre voisins distinctes. La simplification travaille sans récursion et avec
un budget d’opérations. L’arrondi produit de vraies courbes de Bézier.

Le calcul conserve les limites d’ouverture existantes : il ne relève pas la
limite d’image de l’application. La grille est plafonnée à 1 048 576 pixels,
les arêtes à 200 000, les contours à 1 000 et le total à 20 000 nœuds. Un
dépassement est refusé intégralement. Réduire la résolution ou les couleurs,
ou filtrer les petites îles peut rendre une image complexe exploitable.

Cette fonction vise les dessins, logos et aplats. Elle ne reconnaît pas les
objets ou les textes d’une photographie. Palette, seuil, suppression de détails,
simplification, arrondi et résolution réduite sont des approximations visibles
dans l’aperçu. Des couleurs voisines quantifiées et lissées peuvent présenter
des joints aux frontières. Désactiver simplification et arrondi conserve la
silhouette exacte sur la grille, sans promettre une copie universelle du bitmap.

Le mode indexé demande un passage en RVB. Le GIF doit être extrait en image
fixe. La commande ne s’enregistre pas dans les actions/lots, car elle utilise
une révision et une cible de document explicites ; l’hôte refuse cet enregistrement.

## Persistance et exports

Le résultat utilise uniquement les tracés déjà pris en charge par le projet
**v16**, qui reste le format courant, lecture v1–v16. Aucun nouveau format n’est
nécessaire. Le projet garde le bitmap source et sa visibilité, les calques,
les chemins et leurs nœuds. Le SVG n’exporte pas le bitmap source masqué.
Le PDF écrit les contours en géométrie vectorielle, y compris leurs arrondis.

Les moteurs peuvent arrondir l’alpha ou lisser une arête différemment.
La première comparaison d’un arrondi 24 × 24 avec une référence quadratique
observait maximum 48 et moyenne 0,13845. Le critère spécifique est maximum 64,
moyenne 0,2, au plus 16 pixels d’écart alpha supérieur à huit, avec aplats
exacts hors d’une bande d’un pixel. L’export SVG d’un calque à opacité 0,7
observe un écart maximal de un et une moyenne de 0,07324 ; les contrôles de
persistance et de remplacement du bitmap restent exacts dans cette référence.

## Preuves

Huit tests supplémentaires du cœur couvrent notamment tous les **511 motifs
binaires non vides sur 3 × 3 pixels**, une référence RGBA indépendante,
les contours quadratiques arrondis, le filtrage du bruit, la sélection douce,
la réduction de résolution, les refus atomiques, les révisions, l’historique,
les propriétés de calque, la persistance et les exports.

Les contrôles Windows utilisent des bitmaps originaux produits séparément avec
Python/Pillow. L’aperçu couleur, le résultat appliqué, le SVG et son optimisation
ont un écart de pixels nul dans cette référence avec trous et semi-transparence.
Ils vérifient aussi édition des nœuds, clic réel, anciens projets, petites fenêtres,
réponses retardées, refus de bruit excessif, sélection, undo et conservation du source.

Les preuves et sources sont figées dans `outputs/Progression-vectorisation`.
Le jalon reste du développement : les installateurs 1.4 et l’aperçu autonome
v11 précédents sont conservés. L’import PFV et les autres exigences du PLAN restent ouverts.

Validation consolidée : **151 tests du cœur et deux corpus externes**, **443
contrôles Windows**, dont **24 nouveaux**, **19 comparaisons PDF** et **15
inspections structurelles**. Les deux nouvelles références PDF sont vectorielles,
sans image XObject, avec 65 courbes dans la version arrondie. Leurs moyennes
RGB sont 0,21431 et 0,25686, les maxima d’arête 96 et 79. Les aplats ont un
écart nul au-delà de la bande d’arête d’un pixel ; les critères vectoriels
existants restent inchangés (moyenne globale ≤3, aplats maximum 3/moyenne 0,5).
