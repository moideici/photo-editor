# Groupes vectoriels — développement après Prisme 1.4

> État courant : le développement écrit v12 et lit les versions 1 à 12.
> Les jalons de cette page sont historiques ; images et découpes ont été
> complétées par les masques peints dans SVG_MASQUES.md. Les limites des
> motifs complexes et des filtres restent indiquées dans PLAN.md.


Ce travail est dans le source et le binaire de développement. Les installateurs
1.4 déjà livrés restent antérieurs à ces fonctions. Le reste du vectoriel avancé
du plan demeure ouvert. Peintures et motifs simples sont décrits dans PEINTURES.md
et MOTIFS.md ; les nouveaux groupes stylés/importés dans SVG_GROUPES_STYLES.md.

Dans un calque vectoriel, le panneau Formes propose Groupes vectoriels. Cocher
au moins deux formes consécutives d’un même parent, puis Grouper les formes cochées.
Le nouveau groupe est sélectionné et l’outil Déplacement est activé. Le sélecteur
Forme active affiche les groupes et leurs enfants en retrait.

Un groupe se déplace et se redimensionne à la souris ; ses propriétés X/Y,
largeur/hauteur, rotation et opacité sont éditables. Les enfants gardent leur
géométrie et leur style. Un groupe peut être regroupé avec des formes voisines
pour créer une hiérarchie imbriquée. L’opacité s’applique au résultat du groupe,
pas séparément à chaque enfant : les zones qui se recouvrent restent correctes.

Alt+clic sélectionne un enfant ; le sélecteur hiérarchique et les boutons
Sélectionner le premier enfant / Sélectionner le groupe parent permettent aussi
de naviguer. Les coordonnées d’un enfant sont locales à son parent et à sa
transformation affine éventuelle. Une fois un enfant choisi, on peut déplacer
sa géométrie, modifier son style ou éditer ses nœuds. Les autres enfants restent
intacts. Conversion en tracé et booléens restent disponibles dans le même parent.

Dissocier conserve l’ordre, les identités et les styles des enfants. Les matrices
affines reçoivent la transformation du groupe : les formes ne deviennent pas un
bitmap, et un texte reste du texte éditable, même après étirement et rotation.
Une suppression du dernier enfant retire aussi les groupes devenus vides.

Deux restrictions sont explicites :

- Les formes regroupées doivent être consécutives dans la pile. Le déplacement
  implicite d’une forme intermédiaire n’est pas effectué.
- La dissociation exige une opacité du groupe de 100 %. Distribuer une opacité
  de groupe sur ses enfants modifierait leurs recouvrements. La commande refuse
  ce cas sans changer le document ; les opacités propres des enfants sont conservées.
  La dissociation exige aussi une fusion Normal et une isolation désactivée.
  Opacité, fusion et isolation restent éditables sans dissocier.

Le premier jalon de groupes introduisait v7. Le développement de ce jalon écrivait v10
et lit les versions 1 à 10 ; les anciennes versions de Prisme ne peuvent pas
ouvrir ces nouveaux projets. La migration
des anciens projets réattribue les identifiants nuls/dupliqués de formes sans
changer le rendu. Les groupes sont limités à 16 niveaux et 20 000 formes par
calque ; coordonnées/matrices sont contrôlées. Les modifications refusées ne
créent pas d’étape d’historique. Chaque geste réussi reste une seule étape annulable.

Preuves : six nouveaux tests Rust, 15 contrôles dans la vraie application Windows
dans work/phase3-groups-result.json. Sont vérifiés déplacement/redimensionnement,
Alt+clic, style et nœud d’un enfant imbriqué, undo/redo, dissociation sans perte,
migration et persistance, rendu SVG réimporté exact. Les exports SVG et PDF
utilisent la même hiérarchie. Les comparaisons PDF du nouveau jalon de groupes
stylés sont consignées avec les autres peintures dans work/paint-pdf-result.json.
Les régressions plume/SVG et les trois
hôtes de plugins passent. Les empreintes du binaire et des sources sont dans le rapport.
