# Hôte PhotoFiltre PFL expérimental

État du 25 octobre 2026 : **hôte expérimental du jalon 1.4**, validé initialement
dans le binaire de développement. Les installateurs 1.3 restent antérieurs.
Le rapport de livraison indique la validation des paquets 1.4. La comparaison du
même fichier dans PhotoFiltre 11 reste ouverte.
La phase 4 complète n’est donc pas clôturée.

## Utilisation

Outils > Plugins PhotoFiltre PFL affiche le catalogue : charger un fichier .pfl
ou analyser un dossier, choisir un plugin, consulter ses métadonnées et son
icône, ouvrir À propos ou Exécuter. Les menus Filtres, Image et Fichier
filtrent les types utiles. Le délai est réglable de 100 ms à 10 minutes,
120 secondes par défaut. La découverte des dossiers se limite à 200 fichiers.

L’exemple original `plugin-sdk/pfl/examples/negative.pfl` utilise la vraie ABI
PFL de niveau 2 ; son source C et l’en-tête sont sous MIT. Son négatif conserve
l’alpha, respecte la sélection et s’annule en une étape. Une DLL utilisant
l’ABI Prisme, même renommée .pfl, est refusée.

## Contrat compatible

| Sujet | Prise en charge |
|---|---|
| Binaire | PE Windows x64, Unicode UTF-16, au plus 128 Mio |
| Compatibilité | RegisterCompatibility renvoyant 2, 3 ou 4 ; préfixes de structure respectés |
| Exports requis | RegisterCompatibility, RegisterPluginHeader, RegisterPluginStruct, Execute |
| Exports facultatifs | RegisterPluginLanguage, RegisterPluginGlyph, AboutPlugin |
| Glyph | HBITMAP 19 × 19 copié en PNG dans le processus enfant |
| Types | Filtre, Image, Import, Export, Outil, avec restrictions de mutation selon le type |
| Bitmap | DBmp BGR24 avec stride aligné à 4 octets, HBITMAP GDI réel |
| Scanlines | Ligne logique 0 en haut ; pointeur de la ligne suivante à moins un stride |
| Sélection | Rectangle, ellipse, polygone et masque progressif, coordonnées document/calque |
| Alpha | RGB24 conserve l’alpha ; callbacks niveau 4 en alpha inverse (255 = transparent) |
| Indexé | Palette de 2 à 256 couleurs et couleur transparente ; conversion du fond unique |
| Dialogues | Win32 modaux appartenant au processus enfant, PrepareDialog mode 0 |
| Documents | Un document courant ; énumération, noms, composite et miniatures |
| Historique | Une application réussie = une étape ; sortie validée avant adoption |

Le document garde les limites actuelles du moteur : 16 777 216 pixels,
16 384 pixels par côté, 256 calques, budget de projet 512 Mio. Les DBmp
intermédiaires sont limités à 64 et 256 Mio cumulés. `piBitmap` est un jeton
opaque reconnu uniquement par cet hôte. Aucun objet VCL fourni par une DLL
n’est déréférencé. FreeDBmp invalide les pointeurs ; ReleaseHandleDBmp
transfère le HBITMAP au plugin, qui doit ensuite le détruire avec DeleteObject.

SetBitmap/SetSelBitmap écrivent le calque bitmap actif ; les autres calques,
notamment vectoriels, restent intacts. Le masque progressif n’est appliqué
qu’une fois. Le redimensionnement SetBitmap est réservé au type Image avec
un fond RVB opaque unique. NewImage accepte Import/Outil quand aucun document
n’est ouvert : il ne remplace pas silencieusement le document courant.

Le chemin .prisme et son état sauvegardé sont conservés. SetImgFileName sert
au nom de sortie d’un export, sans déclarer le projet sauvegardé. Les buffers
des calques, sélections et EXIF inchangés retrouvent leur partage mémoire au
retour du processus enfant, pour ne pas gonfler l’historique.

## Limites explicites

- ShowForm attend des objets Delphi/VCL incompatibles : refus avec diagnostic.
- SetImageDPI : refus ; GetImageDPI renvoie 72 ppp. Le pipeline DPI/couleur complet
  reste à réaliser, sans simuler une modification de résolution.
- SetApplicationState : refus de la modification de l’état global de l’éditeur.
- PrepareDialog modes 1/2 : aperçu ou pipette liés à l’éditeur non implémentés.
  Un plugin peut gérer son propre aperçu dans son dialogue Win32 modal.
- Pas d’hôte 32 bits, d’ANSI historique, d’API future ou de compatibilité VCL.
- Pas de navigation entre plusieurs documents ni de création à côté d’un document ouvert.
- Calque vectoriel actif : rastérisation explicite nécessaire pour un traitement de pixels.
- Les PFL ne disposent pas ici d’un contrat générique de paramètres persistants :
  exécution refusée pendant l’enregistrement d’une action, pas de rejeu en lot annoncé.
  Les plugins Prisme/Rhai et les 8BF possèdent leurs propres contrats de rejeu.

Les capacités déclarées dans les flags du plugin sont vérifiées avant Execute.
Les callbacks non pris en charge sont interceptés ; leur emploi annule toute
la transaction, même si Execute renvoie ensuite un succès. On ne peut pas
détecter statiquement tous les callbacks qu’une DLL appellera.

## Isolation et preuves

Métadonnées, icône, À propos et Execute chargent la DLL seulement dans un
processus enfant. Un Job Windows limite la mémoire privée à 512 Mio par
processus et termine les descendants. Délai, crash, résultat invalide ou erreur
de callback conservent pixels et historique du document parent. Cette isolation
protège la stabilité de l’éditeur ; elle n’est pas un bac à sable de sécurité.

`work/phase4-pfl-result.json` : **46 contrôles réussis**, 46 callbacks exercés
avec succès, trois callbacks explicitement refusés. Le rapport lie les résultats
aux SHA-256 du binaire et des sources. Il couvre les quatre types avec image,
l’import sans document, les métadonnées isolées, les préfixes API 2/3, les vrais
handles GDI, l’orientation/stride, l’alpha et les masques, undo/redo/projet,
les dialogues, erreurs, annulation, crash, délai et l’utilisation après échec.
Les offsets et leurs références sont dans PFL_ABI.md ; le probe C comporte
22 assertions de taille/alignement.

Une référence indépendante, DemoC de P@F dans le SDK officiel 1.1 (mars 2005),
a été portée en x64 : structures et Unicode, conventions MSVC et fermeture du
dialogue par timer pour l’automatisation. La fonction de traitement Affiche
reste identique. Les 3 072 pixels du résultat sont comparés exactement, puis
une sélection progressive est vérifiée. Provenance et empreintes :
`work/pfl-independent/provenance.json`. C’est un port de référence, pas un
plugin tiers x64 publié sans modification. SDK, ressources et port de référence
restent dans work et ne sont pas redistribués.

La tentative de comparaison dans PhotoFiltre 11 n’a fourni aucune fenêtre
contrôlable. Ce contrôle reste ouvert ; il n’est pas remplacé par les fixtures
du même hôte. Le futur pack PFL original et son contrôle croisé restent aussi
au plan, ainsi que les essais sur les binaires empaquetés.
