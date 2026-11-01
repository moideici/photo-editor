# SDK des filtres Prisme — ABI v1

Ce SDK est sous la licence MIT du projet. Il décrit l’API propre à Prisme ; il
ne constitue pas un hôte Photoshop 8BF ou PhotoFiltre PFL.

Le panneau Photoshop 8BF utilise un hôte et une ABI distincts : voir ../8BF.md.
Le dossier pfl/ contient le contrat PhotoFiltre 11 Unicode x64 et un exemple PFL
original exécuté dans l’hôte de développement. Voir ../PFL.md pour le sous-ensemble
pris en charge et ../PFL_ABI.md pour les structures. Une DLL utilisant prisme_filter_v1 ne devient pas un plugin
PhotoFiltre en changeant son extension.

Un plugin est un fichier `*.prisme-plugin.json` et son script `.rhai` ou sa DLL
Windows x64, dans le même dossier ou un sous-dossier. Le catalogue ne charge
aucune DLL. L’exécution crée un processus hôte séparé pour chaque application.
Les sorties sont validées avant de remplacer les pixels : erreur, plantage ou
délai dépassé conservent le calque et l’historique. Une réussite donne une étape
annulable, respecte la sélection et peut être enregistrée dans une action ou un lot.

## Manifestes

Voir `examples/brightness.prisme-plugin.json`. `schemaVersion` vaut 1 ; `id` est
un identifiant ASCII (lettres/chiffres/point/tiret/underscore), `name` est le nom
affiché, `version` la version de l’auteur, `kind` vaut `rhai` ou `native`, et
`file` désigne le code relatif au manifeste. Pas de chemin absolu ou de `..`.
`parameters` contient au plus 32 réglages numériques `key`, `label`, `min`, `max`,
`default`, `step`. Les clés doivent être distinctes. Une valeur absente prend sa
valeur par défaut ; une valeur inconnue/hors bornes est refusée.
`timeoutMs` vaut 30 000 par défaut et accepte 100 à 120 000 ms.

Les actions conservent le chemin du manifeste et les paramètres. Le code du plugin
doit rester à ce chemin ; une action ne l’incorpore pas et ne fige pas sa version.

## Scripts Rhai

Le point d’entrée est `fn pixel(x,y,r,g,b,a,params)` et doit retourner quatre
nombres `[r,g,b,a]`. Coordonnées entières locales au calque, canaux RGBA8 droits,
paramètres numériques flottants. Les nombres sont arrondis et bornés à 0–255.
L’alpha peut être modifié. Les nombres non finis et les retours incorrects sont
refusés. Les pixels sont traités de gauche à droite, de haut en bas.

L’API ne fournit ni fichiers, ni réseau, ni imports de modules. `eval` est désactivé.
Chaque appel est limité à 10 000 opérations, 32 niveaux d’appels, tableaux/maps
de 64 éléments et chaînes de 4 096 octets. Le processus a aussi un délai global.
Les limites sont appliquées par [Rhai](https://rhai.rs/book/safety/max-operations.html).
Les scripts simples sont adaptés aux traitements par pixel ; les filtres coûteux
sur de grandes images bénéficient de l’ABI native.

## DLL C ABI

Inclure `prisme_plugin.h` et exporter exactement `prisme_filter_v1`. Le fichier
`native/invert.c` est un exemple complet, à compiler dans une invite MSVC x64 :

```bat
cd plugin-sdk\native
cl /nologo /std:c11 /LD /O2 invert.c /link /OUT:invert.dll
```

Le jalon 1.4 fournit aussi cette DLL compilée x64 avec son manifeste dans native/.
Elle peut être chargée directement depuis le panneau Plugins Prisme.

Le contrat est dans le fichier d’en-tête. Le tableau est RGBA8 non prémultiplié,
sans padding entre lignes (`stride = width*4`). Modifier uniquement les pixels.
Les paramètres sont un objet JSON UTF-8 avec une longueur explicite. Ne pas
conserver les pointeurs, ne pas libérer la mémoire de l’hôte et ne pas modifier
la taille ou la géométrie. Retourner 0 en cas de succès ; un autre code produit
une erreur et peut fournir un message UTF-8 dans le tampon borné.

La DLL et ses dépendances doivent être disponibles avant l’exécution. Les
initialiseurs et finaliseurs de DLL s’exécutent également dans l’hôte séparé.
Les DLL sont du code natif avec les droits de l’utilisateur : l’isolation protège
l’éditeur des plantages, elle ne constitue pas une sandbox de sécurité. Le Job
Windows ferme les descendants avec l’hôte et borne sa mémoire privée à 512 Mio.
L’hôte reçoit une copie des pixels et l’éditeur applique ensuite son masque de
sélection. Aucun callback VCL ni objet Photoshop n’appartient à cette ABI.

Le chargement suit le contrat de [libloading](https://docs.rs/libloading/latest/libloading/struct.Library.html).
