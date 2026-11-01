# Images dans les filtres SVG — développement v14

Ce jalon ajoute `feImage` aux seize familles documentées dans SVG_FILTRES.md.
Les anciens installateurs 1.4 et l’aperçu portable v11 restent figés.

## Utilisation

Après décomposition d’un SVG compatible, sélectionner le groupe filtré, ouvrir
« Filtres et effets SVG », puis l’étape « Image intégrée au filtre ». Le panneau
« Source de l’image du filtre » présente son aperçu natif et son contenu.
Une nouvelle étape feImage peut également être ajoutée depuis le catalogue.

Une source peut contenir une image PNG, JPEG, GIF ou WebP intégrée, un SVG
intégré ou une copie de la géométrie visée par une référence locale du SVG.
Elle reste autonome : modifier cette copie ne modifie pas les autres objets.
Les données bitmap originales sont conservées ; l’édition d’un GIF utilise
l’image prise en charge par le moteur SVG, sans animation dans le filtre.

Le chargement propose contenir, recadrer ou étirer, ainsi que neuf alignements.
Ces réglages s’appliquent au prochain chargement, qui remplace le brouillon.
Le remplacement d’une image existante garde sa géométrie et son interpolation.
Le contenu expose position, dimensions, rotation, opacité, matrice et ordre ;
les formes ont leurs peintures et leurs nœuds. Les groupes gardent fusion,
isolation, découpes, masques et filtres. On peut ajouter un rectangle ou une
ellipse et retirer un contenu lorsqu’il reste un autre objet dans son parent.

Les aperçus sont asynchrones ; une ancienne réponse ne remplace pas le dernier
réglage. Pendant le chargement, l’application du filtre est désactivée. Un
chargement terminé après changement de forme ne remplace pas sa nouvelle cible.
Tout reste en brouillon jusqu’à « Appliquer les effets SVG », en une étape
annulable. Annuler les réglages conserve le document et son historique.

## Import, projet et export

Le projet écrit v14 et relit les versions 1 à 14. Les ressources sont des objets
validés, avec identités locales ; leurs références ne polluent pas les identités
des formes principales. Une véritable archive v13 du jalon précédent est
migrée et comparée au rendu natif.

Les repères, régions, opacités et interpolations du filtre restent conservés.
Une source SVG dont l’ombre déborde de son cadre garde ce débordement ; elle
n’est pas découpée arbitrairement lors de sa conversion en groupes éditables.
Les éclairages présents dans un SVG source gardent leurs entrées normalisées.

Le SVG exporté garde la structure du filtre et ses ressources. Les références
locales sont sérialisées sous des identifiants propres à l’export. Le PDF
prépare les sous-arbres filtrés avec le moteur natif à 96 pixels par pouce,
dans une copie du document. Il conserve le contexte du cadre SVG pour éviter
de déplacer la grille d’échantillonnage de feImage. Les objets voisins sans
filtre restent vectoriels. Les effets échantillonnés ont une résolution finie.

## Refus et limites

- Une URL externe n’est pas téléchargée. Une référence absente, des données
  corrompues ou une source éliminée par la normalisation empêchent la
  décomposition : le bloc original reste conservé avec une explication.
  Les définitions inutilisées ne bloquent pas à elles seules la conversion.
- Les références héritées entre définitions de filtres sont suivies avec une
  limite explicite. Les primitives inconnues restent refusées.
- Ressources : huit niveaux de chaînes imbriquées, profondeur de géométrie
  seize, mille formes et vingt mille nœuds par budget de validation de chaîne.
  Les limites générales des formes, peintures, masques et filtres s’appliquent
  également. Un contenu vide ou invalide ne modifie pas le document.
- Images : au plus 12 Mio de données, 16 millions de pixels et 16 384 pixels
  par côté. Le chargement et le PDF conservent leurs budgets explicites ;
  le PDF est limité à 16 millions de pixels préparés cumulés.
- L’édition graphique directe de ces sources sur le canevas et leur livraison
  dans une nouvelle distribution restent ouvertes. Le texte sur tracé, les
  polices variables, la vectorisation et l’import PFV restent au PLAN.

## Vérification

Les preuves de ce jalon sont figées dans
`outputs/Progression-feImage-SVG/validation.json` : tests du cœur, parcours
Windows, références indépendantes, projets, exports comparés et sources.
Les résultats consolidés sont consignés dans EXECUTION.md et BILAN_ACTUEL.md :
127 tests du cœur, deux corpus externes, 370 contrôles Windows (21 nouveaux),
14 comparaisons PDF et dix inspections structurelles. Les références natives
bitmap et vectorielle importées ont un écart nul ; le réimport du SVG exporté
testé a également un écart nul. Les six nouveaux tests du cœur couvrent aussi
les quatre formats bitmap, les ajustements, les unités, les SVG imbriqués,
les refus atomiques, les budgets, le PDF et la sélection native.

Les comparaisons PDF ne sont pas une identité stricte. Les deux nouveaux PDF
bitmap ont une moyenne RGB de 0,6978, un maximum aux arêtes de 209, et 0,2481 %
des pixels au-delà de 32 niveaux. Le premier contrôle de transitions douces
à un pixel trouvait six pixels à plus de huit, maximum 12. L’échantillonnage
du filtre puis de l’image/alpha PDF étend la transition : une bande de deux
pixels donne un maximum de 7 et une moyenne de 0,3998, sous les limites 8 et
0,5. Les aplats constants 3×3 gardent un maximum de 2 sous la limite 3.
Les douze autres scènes gardent leur classement des transitions ; les anciens
seuils ne sont pas élargis. La référence vectorielle feImage passe avec une
bande d’un pixel, maximum 6 dans les transitions douces et 1 dans les aplats.
Le rapport conserve les valeurs du premier classement et les empreintes.
