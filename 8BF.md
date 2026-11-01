# Filtres Photoshop 8BF — jalon Prisme 1.4

## Utilisation

Outils > Filtres Photoshop 8BF ouvre le panneau. Charger un .8bf/.eff ou un dossier,
puis choisir le filtre : un fichier peut contenir plusieurs ressources PiPL.
Le catalogue lit les métadonnées sans charger la DLL. Les entrées incompatibles
restent identifiées avec leur raison ; un binaire 32 bits ne s’exécute pas dans
l’hôte x64.

Sélectionner un calque bitmap, demander éventuellement le dialogue du filtre,
puis Appliquer. Les couleurs de premier/arrière-plan de l’atelier sont transmises.
Le délai maximal est réglable de 0,1 à 600 secondes, 120 secondes par défaut,
dialogue compris. Le bouton À propos utilise également un processus séparé.

Une réussite produit une étape annulable. Une annulation, erreur, sortie invalide,
interruption ou dépassement du délai conserve le document. Les pixels RGBA8,
l’alpha et la sélection sont traités par le cœur ; le masque progressif n’est
pas composé une seconde fois par Prisme. Les coordonnées du masque tiennent
compte de la position du calque dans le document.

## Paramètres, préréglages et actions

Après une réussite, les paramètres effectivement renvoyés sont disponibles dans
le panneau. Enregistrer/charger un fichier .prisme-8bf-preset conserve ces paramètres.
Le nom et la ressource du filtre sont vérifiés au chargement ; le chemin peut
changer, mais le plugin et sa version doivent rester compatibles avec son état.

Pour les filtres déclarant une terminologie de script et renvoyant un descripteur
non vide, Prisme préfère ce descripteur public au bloc binaire opaque. Cela permet
notamment à Filter Foundry d’initialiser ses handles dans chaque nouveau processus
et de reconstruire les formules, sans réutiliser ses globals privés.

Pour les autres filtres, le bloc opaque et l’éventuel descripteur sont conservés.
Certains plugins stockent des pointeurs ou un état privé dans ces blocs : leur
portabilité entre processus dépend du plugin. La compatibilité est vérifiée par
filtre, sans promettre le rejeu de tous les 8BF existants.

Une action enregistre les réglages renvoyés, sans demande de dialogue ; elle peut
être sauvegardée, rejouée sur un autre calque et utilisée en lot. Une action dont
le fichier demande de rouvrir un dialogue est refusée à la lecture. La validation
du document et de l’action est transactionnelle : un état non enregistrable
n’ajoute pas les pixels au document. Un rejeu complet reste une seule annulation.

## Processus hôte et limites

Le programme enfant reçoit une copie du calque et de son masque, applique le filtre
et écrit sa réponse dans un dossier temporaire privé à l’exécution. Seul cet enfant
charge le code étranger. Le parent vérifie dimensions, longueur, profondeur et
état avant de modifier son document. Le Job Windows borne à 512 Mio la mémoire
privée de chaque processus et ferme les descendants à sa fermeture.

Cette isolation protège l’éditeur des plantages ; elle ne réduit pas les droits
Windows du code natif. Les limites actuelles restent RGBA 8 bits, 16 777 216 pixels,
16 384 pixels par côté. Les conversions ICC/16 bits sont un autre jalon du plan.
La résolution passée par défaut est 72 ppp : le modèle de document ne porte pas
encore un réglage général de résolution destiné à tous les outils.

Les suites couvertes sont celles de vendor/eightbf-core. Une découverte PiPL réussie
ne garantit pas que toutes les suites Adobe demandées par un filtre soient prises
en charge. Le plugin peut imposer son interface même lorsqu’aucun dialogue n’est
demandé ; le délai reste la protection contre un appel qui ne rend pas la main.

## Vérifications effectuées

- TypeScript/Vite : compilation réussie.
- Cœur Prisme : 79 tests réussis ; deux corpus externes à relancer au contrôle final.
- Cœur 8BF : 145 tests réussis après l’adaptation du sélecteur Parameters.
- work/phase4-eightbf.cjs : 32 contrôles natifs sur le binaire de développement.
- Fixture C séparée : treize filtres dans un PE x64 avec ressources PiPL réelles,
  paramètres opaques/descripteurs, PICA, plans séparés, advanceState, masque,
  dialogue natif, À propos, erreur, annulation, crash et délai.
- Sélection progressive et calque décalé, alpha partiel, undo/redo, préréglages,
  actions sauvegardées et lots réellement exécutés.
- Filter Foundry 1.7.0.25 x64 original : 3 072 pixels d’un dégradé comparés
  exactement au négatif attendu ; préréglage, action et lot rejoués dans des
  processus différents. Le binaire tiers n’a pas été modifié.
- Régression : plugins Prisme/Rhai, parcours natif, jalon 1.2 et SVG éditable repassés.

Rapport : work/phase4-eightbf-result.json, avec SHA-256 du binaire Prisme et du filtre
tiers ; capture : work/phase4-eightbf.png. Les fixtures se compilent avec
python work/build-eightbf-fixtures.py (MSVC x64 et Windows SDK).

Source du filtre tiers : https://github.com/danielmarschall/filter_foundry,
releases/FilterFoundry 1.7.0.25 (SVN Rev 624).zip. Licence GPL-2.0-or-later conservée
dans work/8bf-reference ; utilisé uniquement comme référence de test, non inclus
dans les bundles ou le SDK Prisme. Son empreinte figure dans le rapport.

Le composant hôte est sous MIT, issu de distortionary-labs/8bf-host à la révision
2a52154f160bb25c9ab2a77c29c7ddaccf0b8258. L’adaptation Prisme du sélecteur
Parameters évite d’ouvrir le dialogue quand un descripteur de script valide
permet le rejeu sans interface ; la préférence de rejeu va au descripteur public
pour les filtres déclarant leur terminologie. Provenance détaillée dans
vendor/eightbf-core/UPSTREAM.md. La fixture C amont et sa licence sont conservées
dans src-tauri/tests/fixtures/eightbf_upstream.*.

Ces changements ne sont pas dans les installateurs 1.3 existants.
Le PFL expérimental est décrit dans PFL.md. La phase 4 complète reste ouverte :
comparaison PFL dans PhotoFiltre et plugin PFL tiers x64 publié à vérifier.
Le rapport de livraison indique les essais de chaque famille dans les paquets.

