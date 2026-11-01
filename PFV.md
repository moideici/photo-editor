# Tracés PhotoFiltre PFV — import de géométrie

Jalon du 27 octobre 2026, complément de I.2. Un PFV est ouvert depuis Fichier >
Ouvrir, ou ajouté comme calque vectoriel depuis Calque > Importer un tracé
PhotoFiltre PFV. Le résultat est un vrai chemin éditable : sommets, poignées,
trou/fermeture et annulation suivent les outils vectoriels existants.

## Contrat observé

PFV contient la géométrie d'un tracé, sans couleurs, canevas ni calques. Prisme
attribue un trait noir de 1 px, sans fond. L'ouverture recadre le cadre des
sommets et poignées avec une marge de 16 px et signale la translation dans les
informations du document. L'import dans un document garde les coordonnées du
fichier, crée un calque, conserve la sélection et donne une seule étape d'undo.
Les coordonnées négatives sont acceptées dans les limites ci-dessous.

| Offset | Contenu vérifié |
|---|---|
| 0–15 | Les 16 octets ASCII `PhotoFiltre Path`, sans zéro ajouté. |
| 16–47 | 32 octets réservés nuls dans le format observé. |
| 48 | i32 LE : mode Bézier 0/1. |
| 52 | i32 LE : fermeture 0/1. |
| 56 | i32 LE : nombre de sommets. |
| 60… | Mode droit : deux i32 par sommet ; mode Bézier : huit i32 par sommet. |

Les huit valeurs sont ancre x/y, poignée entrante x/y, indicateur entrant,
poignée sortante x/y, indicateur sortant. Un indicateur **1** donne une poignée
explicite ; **0** demande une poignée automatique, au tiers du déplacement vers
l'ancre voisine, avec arrondi entier au plus proche et ties-to-even. Aux extrémités
ouvertes sans voisin, la poignée reste sur l'ancre. Le bouclage automatique est
réservé à un tracé fermé de plus de deux sommets. Les poignées sont converties
en points explicites modifiables dans Prisme.

Le lecteur exige une longueur exacte `60 + nombre × 8` ou `60 + nombre × 32`,
entre 2 et 20 000 sommets et au plus 640 060 octets. Tous les indicateurs doivent
être 0/1 ; toutes les coordonnées, même inutilisées, restent dans ±1 000 000.
Signature, réserves, variante, longueur ou coordonnées inconnues sont refusées
avec un message explicite, sans import partiel et sans mutation de l'historique.
L'ouverture garde aussi les limites générales de dimensions du document.

Il s'agit du **sous-ensemble binaire observé**, pas d'une spécification officielle
complète de tous les futurs PFV. Aucun export PFV, import PFL ou format propriétaire
supplémentaire n'est déduit de cette fonction. Le PFV ne contient pas un masque
bitmap ou une sélection : ces objets ne sont pas inventés à l'ouverture.

## Références et validation

La documentation PhotoFiltre présente la sauvegarde des tracés PFV :
[tracés vectoriels](https://photofiltre-studio.com/doc/vpath.htm).
Le contrat binaire a été confronté à dix fichiers réels du dossier Paths de la
distribution officielle `pfsx-fr-10.14.2.zip`, déjà présente dans les références
locales, et aux routines de lecture/écriture de la référence PhotoFiltre :
lecteur 0x8bd750, écrivain 0x8bd560, nœuds 0x8bd2e0/0x8bd440,
poignées 0x8ba950/0x8ba9b0 et calcul automatique 0x8baa10.

Les dix fichiers (Courbe 01–03, Divers 01–03 et PF-Carreau/Coeur/Pique/Trefle)
sont comparés dans l'application Windows à un lecteur Python indépendant qui
construit du SVG. Les nombres de sommets et tous les pixels comparés concordent
dans ce corpus ; les métriques et SHA-256 sont consignés dans le rapport.
Ce contrôle n'est pas une comparaison visuelle avec une fenêtre PhotoFiltre.

Quatre tests Rust vérifient aussi lignes et courbes, poignées explicites et
automatiques, ouverture/import, persistance du projet et refus de corruption.
Une fixture PFV originale est livrée pour reproduire les essais sans redistribuer
les ressources PhotoFiltre. Les dix PFV de référence et leurs SVG dérivés restent
hors des livrables. Voir `make-experience-fixtures.py` et les rapports du nouveau
dossier de preuves. Une vraie archive v16 indépendante est également migrée en
projet v17 lors de la validation de l'expérience.

I.2 conserve ses autres exigences ouvertes : cette fonction seule ne termine
pas l'ensemble du vectoriel avancé.
