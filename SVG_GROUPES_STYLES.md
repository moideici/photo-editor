# Groupes SVG : opacité, fusion et isolation

> État courant : le développement écrit v12 et lit les versions 1 à 12.
> Les jalons de cette page sont historiques ; images et découpes ont été
> complétées par les masques peints dans SVG_MASQUES.md. Les limites des
> motifs complexes et des filtres restent indiquées dans PLAN.md.


État : développement après 1.4, sans nouvel installateur. La distribution 1.4
reste figée. Ce jalon complète les groupes de GROUPES.md ; il ne termine pas
tous les SVG complexes du plan.

« Ouvrir un SVG éditable… » et « Décomposer le SVG en tracés » conservent maintenant
les groupes qui portent une opacité, une fusion ou une isolation. Leurs enfants
restent des tracés éditables, avec dégradés et motifs simples. Les groupes
neutres sont dépliés. Texte et traits importés deviennent des surfaces ; ce
jalon ne prétend pas préserver le texte SVG en texte modifiable.

Dans Formes > Groupes vectoriels, choisir Fusion du groupe et l’option
Isoler la fusion des enfants. Les 17 modes SVG sont disponibles : Normal,
Produit, Superposition, Incrustation, Obscurcir, Éclaircir, Densité couleur −/+, 
Lumière crue/tamisée, Différence, Exclusion, Teinte, Saturation, Couleur,
Luminosité et Lumière additive. L’opacité reste dans les propriétés de la forme.

L’opacité et la fusion s’appliquent au résultat du groupe entier. L’isolation
limite les mélanges des enfants au contenu du groupe. Un enfant reste
sélectionnable par Alt+clic ou dans le sélecteur hiérarchique ; sa couleur,
sa géométrie et ses nœuds restent modifiables. Les coordonnées sont locales
à son parent. Déplacement, étirement, undo/redo et projets gardent les styles.

La fusion des enfants d’un calque est isolée du contenu des autres calques,
comme dans le rendu natif. L’export SVG porte explicitement cette isolation
uniquement si le calque contient des groupes avec fusion. Cela évite d’ajouter
une surface et des arrondis alpha aux documents qui n’en ont pas besoin.

Une dissociation qui changerait le rendu est refusée : opacité du groupe à 100 %,
fusion Normal et isolation désactivée sont nécessaires. Les styles propres des
enfants restent conservés. Répartir une opacité sur des enfants qui se recouvrent
ou effacer leur contexte de fusion changerait l’image ; aucune rastérisation
implicite ni suppression de style n’est effectuée.

Les imports, décompositions et duplications attribuent une nouvelle identité
à tous les descendants. Les arbres invalides sont refusés avant adoption.
Le projet de ce jalon était **v10**, lecture des versions 1 à 10. Un projet v9 est
relu puis réenregistré au rendu exact dans le contrôle natif. Les limites restent
16 niveaux de groupes éditables, 20 000 formes par calque ; la décomposition SVG
produit au plus 1 000 surfaces et 20 000 nœuds, avec 64 niveaux SVG neutres au maximum.

Les images intégrées, filtres, masques et découpes restent refusés à la
décomposition, y compris si d’autres éléments du fichier étaient convertibles.
Le document et l’historique restent intacts. Le fichier peut rester un bloc SVG
rendu ; ces structures restent à rendre éditables dans la suite du plan.
Les groupes stylés à l’intérieur d’un carreau de motif restent aussi ouverts.
En PDF, Lumière additive est refusée explicitement par le convertisseur actuel,
en plus des peintures déjà exclues dans PEINTURES.md ; le SVG reste exportable.

Preuves : six tests Rust spécifiques couvrent les 17 modes avec/sans isolation,
opacité d’un bloc sur plusieurs surfaces, placement affine, enfants, projets,
identités, refus et séparation des calques. Le parcours Windows compte 19
contrôles : work/phase3-styled-groups-result.json. Sur la référence tournée avec
transparence et dégradé, l’écart prémultiplié maximal est 12/255, moyen 0,06171 ;
seuls deux pixels sur 153 600 dépassent 8 niveaux. Ce sont des différences de
couverture de bord après conversion en tracés. Les seuils natifs sont 16, 0,15
et moins de 0,01 % des pixels au-dessus de 8. Le SVG exporté/réimporté de la
fixture est exact. Les empreintes et métriques brutes sont conservées.

Le PDF de la même fixture est rendu par Poppler sur papier blanc à 96 dpi :
écart RGB moyen 0,51760/255, maximum 34 sur des bords. Dans les zones éloignées
d’un bord, moyenne 0,43772 et maximum 3. Les mesures et seuils sont dans
work/paint-pdf-result.json ; le rendu a aussi été inspecté visuellement.

Le parcours vérifie aussi un correctif des champs numériques : quitter un champ
vide restaure sa valeur, et une tabulation sans modification ne doit ni arrondir
une coordonnée décimale ni créer d’historique. Les saisies effectivement modifiées
restent bornées et appliquées normalement. C’est le correctif P04 du bilan des
trente propositions ; P07 reste ouvert pour les autres commandes sans effet.
