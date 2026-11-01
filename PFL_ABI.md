# Audit PFL Unicode x64 — structures et cartographie

État : **cartographie statique vérifiée et hôte de développement exécuté**.
Ce document décrit les preuves de structure ; les preuves comportementales
actuelles et les limites de compatibilité sont dans PFL.md et
work/phase4-pfl-result.json (46 contrôles natifs réussis).
La comparaison dans PhotoFiltre et les essais empaquetés restent ouverts.

## Sources et reproduction

- Documentation officielle : https://www.photofiltre-studio.com/plugins/plgref_en.htm.
- SDK officiel 1.1, mars 2005, téléchargé par le lien « Development kit » :
  https://www.photofiltre-studio.com/plugins/addones/SDK-Plugins.zip.
  Sources de référence conservées uniquement dans work/pfl-sdk-reference.
- Binaire PhotoFiltre 11 x64 du dossier voisin : empreinte
  959c1a229d74fde4b5960437158a92cfe5dd382200b47c534e83db1ab0d09987.
- ../photofiltre/analysis/pfstruct.asm : écritures réellement effectuées par la
  fonction 0x912860 dans la structure fournie par le plugin.

work/map-pfl-callbacks.py lit l’ordre des callbacks du SDK Pascal, le rapproche
des paires LEA/écriture observées et produit work/pfl-callbacks/mapping.json/.md
et un désassemblage de travail par cible. Les empreintes du SDK et du binaire
sont conservées dans le JSON. Résultat : 49 callbacks, répartis 27 + 5 + 7 + 10 ;
la case 0x170 est réservée et n’est pas un callback.

Le fichier plugin-sdk/pfl/prisme_pfl.h contient des déclarations C originales
de ce contrat fonctionnel, sans reprendre le code du SDK ou de PhotoFiltre.
Il sert aux plugins et aux fixtures de l’hôte de développement ; aucun hôte
PFL n’est encore livré dans les installateurs 1.3. Le code propriétaire et les exemples officiels téléchargés
ne sont pas inclus dans le bundle ou ce SDK Prisme.

## Corrections par rapport aux premières hypothèses locales

1. **La sélection précède l’image dans TPluginStruct.** Offset 0x10 = SelRect ;
   offset 0x20 = ImgRect. Le premier relevé PLUGIN_API.md voisin les nommait
   dans l’ordre inverse, avec un niveau d’inférence. Le SDK PlgInt.pas et le
   code GetSelBitmap confirment cette correction : celui-ci appelle 0xA0E500,
   le même calcul de rectangle écrit dans la case 0x10.
2. **TPlgImage est aligné naturellement en x64.** CreateDBmp écrit piSize=0x38,
   puis piBuffer à 0x20, piBitmap à 0x28 et piHandle à 0x30. Les sept entiers
   occupent 28 octets, suivis de quatre octets d’alignement. Une déclaration
   packed de 52 octets dérivée littéralement de la documentation ancienne
   aurait placé les pointeurs au mauvais endroit.
3. **ReleaseHandleDBmp renvoie un HBITMAP.** La signature du SDK Pascal est
   TReleaseHandleDBmpProc = function(...): HBitmap ; la cible x64 0x9117D0
   renvoie également la valeur dans RAX. Ce n’est pas simplement un booléen.
4. **Les flags de formats ne suivent pas immédiatement le bit RGBA.**
   Le SDK donne JPEG=0x00010000, GIF=0x00020000, PNG=0x00040000,
   BMP=0x00080000, TIFF=0x00100000, PFS=0x00200000, EXIF=0x00400000.
   Les bits 0x1000 à 0x8000 ne doivent pas être inventés à partir de la liste
   descriptive des options.
5. Le callback de base GetSelMask (0x0A8) lit le masque à la taille de l’image ;
   le callback de niveau 3 GetSelSelMask (0x138) le lit à la taille du rectangle
   de sélection. Les helpers Delphi leur donnent des noms proches.
6. Les LongBool de réussite observés valent souvent -1 ; accepter tout non-zéro.
   La couleur Delphi TColor est un entier COLORREF avec valeurs spéciales signées.
7. GetScanlineDBmp(0) est la première ligne logique en haut ; les lignes
   suivantes reculent d’un stride dans le DIB bottom-up. La cible observée
   0x5CC740 et la boucle du SDK C confirment cet ordre, testé dans Prisme.
8. GetSelShape renvoie -1 pour le rectangle et -2 pour l’ellipse ; les valeurs
   positives donnent le nombre de sommets du polygone. CL_NONE vaut 0x1FFFFFFF.

## Structures de référence

| Structure ou champ | Taille / offset x64 |
|---|---|
| Header | 0x588 / 1 416 octets |
| Header.PluginType | 0x100 |
| Header.Flags | 0x584 |
| Language.Code / Name | 0x00 / 0x08 ; zone connue 0x88 octets |
| Image DBmp | 0x38 / 56 octets |
| DBmp.Buffer / Bitmap / Handle | 0x20 / 0x28 / 0x30 |
| Host.MainHandle | 0x00, pointeur 8 octets |
| Host.ImageExists / Selected | 0x08 / 0x0C, LongBool 32 bits |
| Host.SelRect / ImgRect | 0x10 / 0x20, quatre entiers 32 bits chacun |
| Host.XP_ThemesEnabled | 0x30 ; alignement à 0x38 |
| Fin des champs écrits par PhotoFiltre 11 | 0x1C8 / 456 octets |
| Host avec dix pointeurs de réserve du SDK | 0x218 / 536 octets |

Les chaînes des structures modernes observées sont UTF-16. Les chaînes et
dimensions du SDK ANSI 2005 ne doivent donc pas être recopiées telles quelles.
Pour les retours de structures, utiliser l’ABI Microsoft x64 : TRect de 16 octets
avec tampon de retour implicite ; TPoint de 8 octets renvoyé en registre.
La structure fournie par le plugin doit réserver assez de mémoire pour le niveau
de compatibilité annoncé. Le nouvel en-tête réserve les quatre niveaux et dix
pointeurs futurs.

## Contrôle C indépendant du code Rust

src-tauri/tests/fixtures/pfl_abi_probe.c compile les déclarations avec MSVC x64,
sans Tauri ni code du futur hôte. Vingt-deux assertions statiques contrôlent
tailles et offsets, notamment chaque début/fin de niveau et la case réservée.

Commande : cmd /c work/build-pfl-abi.cmd.
Rapport : work/pfl-tests/abi-layout.json, passed=true, pointeur=8,
header=1416, language=136, image=56, host=536.
Ce contrôle valide les déclarations de structure, pas le comportement des callbacks.

## Exécution dans Prisme et vérifications restantes

Les callbacks et exports sont maintenant exécutés dans un processus isolé,
y compris les métadonnées. Les 46 contrôles natifs couvrent les vrais DBmp
BGR24/GDI, scanlines, transfert de handle, coordonnées, alpha inverse, masques,
annulation et persistance. Les préfixes API 2 et 3 sont protégés par sentinelles.
ShowForm, SetImageDPI et SetApplicationState sont refusés ; PrepareDialog
fonctionne en mode modal 0 seulement. Le détail de chaque limite est dans PFL.md.

DemoC du SDK officiel, porté en x64 avec algorithme Affiche inchangé, fournit
une référence indépendante de la fixture et du futur pack Prisme. Provenance,
adaptations et SHA sont conservés ; cette référence n’est pas redistribuée.
La comparaison du même .pfl dans PhotoFiltre 11 reste à effectuer, ainsi que
l’essai d’un plugin tiers x64 publié sans adaptation et la validation empaquetée.
Une DLL C ABI Prisme renommée .pfl est effectivement refusée par les tests.

La documentation officielle sur les plugins exécutables externes de PhotoFiltre
11.6 décrit un autre mécanisme PNG/exécutable. Il ne remplace pas l’ABI PFL
promise ici.

## Correspondance des callbacks

L’ordre SDK est raccordé aux adresses réellement écrites par PhotoFiltre.
Le rapport mapping.json conserve runtimeVerified=false : il s’agit d’un relevé
statique de PhotoFiltre, pas d’une preuve d’exécution dans PhotoFiltre. La liste
distincte verifiedCallbacks dans phase4-pfl-result.json identifie les 46 callbacks
exercés avec succès dans Prisme ; elle ne prouve pas toutes leurs variantes.

| Offset | Niveau | Callback SDK | Cible PhotoFiltre 11 |
|---|---|---|---|
| 0x038 | 1 | BeginFilter | 0x90FCD0 |
| 0x040 | 1 | EndFilter | 0x90FD60 |
| 0x048 | 1 | GetBitmap | 0x90FD80 |
| 0x050 | 1 | SetBitmap | 0x90FE50 |
| 0x058 | 1 | GetSelBitmap | 0x910100 |
| 0x060 | 1 | SetSelBitmap | 0x9101F0 |
| 0x068 | 1 | GetTransparentColor | 0x910320 |
| 0x070 | 1 | SetTransparentColor | 0x910380 |
| 0x078 | 1 | GetColorCount | 0x910460 |
| 0x080 | 1 | SetColorCount | 0x910490 |
| 0x088 | 1 | NewImage | 0x910530 |
| 0x090 | 1 | ShowForm | 0x910820 |
| 0x098 | 1 | SetAlphaBlend | 0x910A30 |
| 0x0A0 | 1 | DrawSelBorder | 0x910B80 |
| 0x0A8 | 1 | GetSelMask | 0x910C00 |
| 0x0B0 | 1 | ResetSaveMenu | 0x910DA0 |
| 0x0B8 | 1 | GetImgFileName | 0x910E00 |
| 0x0C0 | 1 | GetExePath | 0x910E80 |
| 0x0C8 | 1 | GetOpenPath | 0x910F00 |
| 0x0D0 | 1 | GetSavePath | 0x910F80 |
| 0x0D8 | 1 | GetPluginPath | 0x911000 |
| 0x0E0 | 1 | GetIniFile | 0x911080 |
| 0x0E8 | 1 | SetImgFileName | 0x9110A0 |
| 0x0F0 | 1 | GetBitmapByName | 0x911150 |
| 0x0F8 | 1 | GetThumb | 0x911260 |
| 0x100 | 1 | GetThumbByName | 0x911350 |
| 0x108 | 1 | EnumImages | 0x9114B0 |
| 0x110 | 2 | PrepareDialog | 0x910900 |
| 0x118 | 2 | CreateDBmp | 0x9115D0 |
| 0x120 | 2 | FreeDBmp | 0x911700 |
| 0x128 | 2 | ReleaseHandleDBmp | 0x9117D0 |
| 0x130 | 2 | GetScanlineDBmp | 0x911770 |
| 0x138 | 3 | GetSelSelMask | 0x911820 |
| 0x140 | 3 | GetImageDPI | 0x9119F0 |
| 0x148 | 3 | SetImageDPI | 0x911A40 |
| 0x150 | 3 | SetSelMask | 0x911AA0 |
| 0x158 | 3 | ResetUndoArea | 0x911CC0 |
| 0x160 | 3 | SetApplicationState | 0x911DA0 |
| 0x168 | 3 | GetSelShape | 0x911EA0 |
| 0x178 | 4 | GetLayerRect | 0x911FF0 |
| 0x180 | 4 | GetLayerBitmap | 0x9120A0 |
| 0x188 | 4 | SetLayerBitmap | 0x912170 |
| 0x190 | 4 | GetLayerAlpha | 0x912250 |
| 0x198 | 4 | SetLayerAlpha | 0x912350 |
| 0x1A0 | 4 | GetSelAlpha | 0x912470 |
| 0x1A8 | 4 | SetSelAlpha | 0x912590 |
| 0x1B0 | 4 | DisableLayersPainting | 0x9126C0 |
| 0x1B8 | 4 | GetImageColorMode | 0x912730 |
| 0x1C0 | 4 | GetLayerPos | 0x9127C0 |
