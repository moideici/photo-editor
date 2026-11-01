# Polices variables et styles de texte — développement, projet v16

Ces fonctions sont disponibles dans le développement après le jalon 1.4.
Les installateurs 1.4 et l’aperçu portable v11 déjà distribués sont conservés.

## Utilisation

Sélectionner un texte vectoriel, puis ouvrir **Police variable et styles**
sous ses réglages. La liste propose les familles réellement installées au
démarrage de l’application. Une police fixe n’affiche aucun axe variable.
Une nouvelle police installée pendant la session nécessite un redémarrage.

Les axes proviennent de la table fvar de la police : nom, tag, minimum,
valeur par défaut, maximum et indication d’axe masqué. Graisse, largeur,
inclinaison et taille optique ne sont proposés que si la police les possède.
Les champs et curseurs partagent un brouillon, avec aperçu calculé par le
moteur natif. **Appliquer les variations** produit une seule étape annulable.
**Annuler les variations** restaure les réglages du texte sélectionné.
Les réponses d’aperçu périmées sont ignorées et les valeurs hors plage refusées.

Une valeur explicite prime sur le réglage automatique correspondant, notamment
Gras pour wght. **Libérer l’axe** retire cette valeur. Pour opsz, Automatique
utilise le corps, borné à la plage de la police ; l’autre choix utilise la valeur
par défaut du dessin. Réinitialiser retire tous les axes explicites et rétablit
la taille optique automatique. Le moteur peut rendre d’autres axes fvar que
les cinq noms traduits dans le panneau ; leur nom fourni par la police est affiché.

La liste des polices du document signale une famille absente et la famille
de repli utilisée. Le nom demandé reste conservé dans le projet. Choisir une
police installée constitue un remplacement explicite et retire les axes
de l’ancienne famille ; undo restaure le choix précédent. Une valeur provenant
d’un autre environnement qui dépasse les plages actuelles reste conservée :
le rendu la borne ou ignore un axe inconnu, avec diagnostic. Le panneau demande
de corriger ou réinitialiser ces axes avant une nouvelle application.

## Styles réutilisables

Enregistrer un style copie famille, corps, gras/italique, axes, taille optique,
fond, trait, épaisseur, opacité, dégradés et motifs. Il ne copie ni contenu,
position, transformation, filtre, masque, découpe ni guide de tracé.
Un nom existant, sans distinction de casse, met son modèle à jour.

Un style s’applique au texte sélectionné ou à tous les textes du calque, y
compris ceux des groupes. Une application multiple est atomique et annulable
en une étape. Les textes conservent leur contenu, position et lien de tracé.
Le corps redimensionne le cadre du texte droit ; les lettres courbes sont
recalculées dans leur repère avec le guide existant.

Les styles sont des modèles par copie : les modifier ou les supprimer ne
change pas les textes auxquels ils ont déjà été appliqués. La bibliothèque
du document est sauvegardée dans le projet et dans l’historique. Elle peut
être exportée/importée en `.prisme-text-styles` (JSON version 1, maximum 4 Mio,
128 styles par document). Les collisions à l’import sont renommées, par
exemple « Titre (importé 2) ». Une bibliothèque invalide est rejetée sans
import partiel. L’export seul n’ajoute aucune étape d’historique.

## Persistance, rendu et limites

Le projet écrit v16 et relit v1–v16. Les anciens projets reçoivent une
bibliothèque vide et aucun axe explicite. Les fichiers de police ne sont
pas intégrés aux projets ou aux bibliothèques : il faut disposer des polices
sur le poste de destination. Les diagnostics restent visibles si elles manquent.
Les extensions PFL conservent la bibliothèque du document courant lorsqu’elles
remplacent ses pixels ; elles n’exposent pas d’API de styles typographiques.

Le texte droit et le texte sur tracé partagent les mêmes axes et le même
moteur. La sélection à la souris teste les pixels des glyphes, y compris au-delà
du cadre nominal et dans les groupes. Le SVG ordinaire conserve le texte et
les propriétés CSS de variations. Le sérialiseur usvg 0.49 omet ces propriétés :
pour un document qui en contient, l’option SVG optimisé conserve donc le SVG
fidèle d’origine ; elle ne garantit pas une réduction ni l’arrondi de précision.
La décomposition d’un SVG conserve le contrat de conversion en contours :
le projet est nécessaire pour retrouver le texte et ses liens éditables.

Le convertisseur PDF plus ancien ne dispose pas du même support des variations.
Les glyphes des polices variables sont préparés en contours vectoriels avec
le moteur natif, dans une copie du document. Les textes du projet restent
éditables. La conversion quadratique/cubique et le lissage de Poppler peuvent
produire des écarts aux arêtes ; les comparaisons publient leurs seuils.
Les limites de texte sur tracé, dont les lettres dépassant la longueur du guide,
restent décrites dans TEXTE_SUR_TRACE.md. Ce jalon n’ajoute pas de mise en page
multiligne, de styles liés en direct ni d’installation de polices.

## Références de validation

Sept nouveaux tests du cœur utilisent une petite police variable originale,
chargée uniquement dans les tests : neuf maîtres, glyphes polygonaux, axes
wght/opsz. Leurs références sont produites séparément avec FontTools.
Les comparaisons vérifient les glyphes, les avances, l’optique automatique,
la priorité des valeurs explicites, les refus atomiques, les styles, l’historique,
la persistance, le SVG et la conservation du document pendant l’export PDF.

Les contrôles Windows utilisent la vraie police Segoe UI Variable installée
ici, dont les axes sont wght 300/400/700 et opsz 5/10,5/36. Les contours FontTools
et une référence textPath indépendante sont comparés au rendu de l’éditeur.
La provenance et l’empreinte de la police système sont consignées sans
redistribuer son fichier. Les résultats consolidés et les sources sont
figés dans `outputs/Progression-polices-styles/validation.json`.

Validation consolidée : **143 tests du cœur et deux corpus externes**, **419
contrôles Windows**, dont **23 nouveaux**, **17 comparaisons PDF** et **13
inspections structurelles**. Les références indépendantes droites et courbes
ont un écart nul. Les deux nouveaux PDF ont des moyennes RGB de 0,25899 et
0,12434, avec maxima d’arête de 96 et 104 ; les aplats au-delà de la bande
d’arête d’un pixel ont un écart nul. Aucune image XObject n’est présente dans
ces deux PDF ; le second contient notamment 169 courbes de Bézier.
