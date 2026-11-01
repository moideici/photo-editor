# Interopérabilité PhotoFiltre

Prisme possède son propre code et sa propre interface. Aucun exécutable, plugin,
visuel, masque ou bibliothèque de formes PhotoFiltre n'est distribué avec Prisme.

## Sélections `.pfs` — phase 1

La documentation officielle décrit PFS comme le format d'enregistrement des
sélections : <https://www.photofiltre-studio.com/doc/selections.htm>.

Grammaire observée dans les sélections de la distribution officielle Studio X
10.14.2 : première ligne = nombre de sommets, puis exactement autant de lignes
`x;y`, avec coordonnées entières et fins de ligne CRLF. L'import accepte aussi
LF, BOM UTF-8 et UTF-16 LE. Aucun code n'est exécuté.

L'import Prisme centre le polygone dans la sélection courante ou dans le document.
Il réduit les formes trop grandes en conservant les proportions. Les tests utilisent
uniquement des polygones synthétiques écrits pour Prisme.

Distribution consultée pour vérifier le format, sans en embarquer les ressources :
<https://www.photofiltre-studio.com/utils/pfsx-fr-10.14.2.zip>.

## Plugins `.pfl` — phase 4, non implémentée

Le dossier voisin `../photofiltre/PLUGIN_API.md` décrit les exports et structures
observés dans PhotoFiltre 11 x64. Il confirme plusieurs points encore à résoudre,
notamment la correspondance des callbacks et le passage d'objets Delphi VCL.
Ces informations préparent le futur hôte isolé ; elles ne prouvent pas que les
plugins sont compatibles avec Prisme aujourd'hui. Les DLL présentes dans ce
dossier ne sont ni chargées ni copiées par Prisme.

Les tracés `.pfv` restent prévus en phase 3.

## Emplacements de PhotoFiltre 11 — jalon 1.2

Référence locale : ../photofiltre/unpacked/PhotoFiltre11.exe, formulaires Delphi
TMAINFORM et TOUTILSFORM. Le relevé de propriétés dans work/photofiltre-layout
est un document de travail ; aucune ressource extraite ne figure dans le bundle.
Les coordonnées des contrôles confirment trois colonnes et six rangées :

| Rangée | Gauche | Centre | Droite |
|---|---|---|---|
| 1 | Sélection | Gestion des calques | Déplacement de la vue |
| 2 | Pipette | Baguette | Ligne |
| 3 | Remplissage | Aérographe | Gomme |
| 4 | Pinceau | Pinceau avancé | Tampon |
| 5 | Flou | Doigt | Retouche |
| 6 | Déformation | Pinceau artistique | Tube |

Prisme garde cet ordre et ajoute ses outils vectoriels et sélections au-dessous.
Les fonctions absentes sont grisées. Deux barres supérieures suivent l’ordre
commandes puis réglages/filtres ; couleurs et options encadrent la palette droite,
avec les calques à gauche. Les dimensions s’adaptent à la fenêtre et ne cherchent
pas à reproduire les pixels, le thème ou les icônes de PhotoFiltre.

Documentation officielle complémentaire :
[palette](https://www.photofiltre-studio.com/doc/palette-outils.htm),
[barres](https://photofiltre-studio.com/doc/barre-outils.htm),
[préférences](https://www.photofiltre-studio.com/doc/pref.htm).
Le lancement par l’outil de contrôle n’a pas fourni de fenêtre PhotoFiltre pilotable ;
le relevé est donc fondé sur les formulaires locaux, sans comparaison visuelle en direct.

Import des tracés PFV réalisé au jalon du 27 octobre : contrat binaire observé,
dix références et limites dans [PFV.md](PFV.md). Cette validation ne remplace
pas la comparaison inter-hôtes des plugins PFL, qui reste ouverte.
