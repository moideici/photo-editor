# Contrôle PFL dans PhotoFiltre 11 — protocole à renseigner

État : **non exécuté dans PhotoFiltre**. Le lancement de la référence locale
n’a pas exposé de fenêtre contrôlable. Les résultats Prisme sont dans
work/phase4-pfl-result.json ; aucun résultat PhotoFiltre n’est présumé.

Utiliser une installation PhotoFiltre 11 x64 autorisée, son dossier Plugins_64
et le même `plugin-sdk/pfl/examples/negative.pfl` original MIT que dans Prisme.
Ne pas utiliser la DLL native Prisme renommée, un plugin 32 bits, ni modifier
le binaire PhotoFiltre. Noter version, SHA-256 du logiciel et du .pfl.
La copie de référence locale connue a l’empreinte
959c1a229d74fde4b5960437158a92cfe5dd382200b47c534e83db1ab0d09987.

Créer dans les deux hôtes une image RVB 64 × 48 de couleur #204060, puis
répéter chaque scénario à partir de cette image propre. Exporter chaque résultat
en PNG, conserver l’original et le résultat, puis comparer les pixels décodés
RGBA (pas les octets compressés des fichiers). Tolérance fixée : zéro pour
ce négatif entier. Comparer séparément dimensions, RGB et alpha.

| Scénario | Résultat attendu du contrat | Résultat PhotoFiltre |
|---|---|---|
| Sans sélection | Tous les pixels #DFBF9F ; alpha inchangé | À renseigner |
| Rectangle x=8, y=8, 16 × 16 | Négatif dans la sélection seulement | À renseigner |
| Ellipse/polygone | Couverture et bords à comparer avec le masque exact | À renseigner |
| Masque uniforme 128/255 | Valeur RGB #808080 avec arrondi au plus proche | À renseigner |
| Calque décalé x=5, y=7 | Région en coordonnées document, autres pixels conservés | À renseigner |
| Alpha partiel | Alpha conservé exactement ; comparer RGB seulement si alpha non nul | À renseigner |
| Autre calque au-dessus | Autre calque et ses propriétés intacts | À renseigner |
| Undo puis redo | Original exact, puis résultat exact | À renseigner |
| Sauvegarde/réouverture | Résultat et pile conservés par chaque format projet natif | À renseigner |

Noter toute différence de masque calculé par l’hôte avant d’attribuer un écart
au plugin. Un PNG aplati ne prouve pas à lui seul la conservation des calques.
Les projets restent propres à chaque application ; seul le rendu RGBA se compare.

La référence DemoC portée x64 utilise aussi PrepareDialog mode 0. Son test
peut être répété localement avec le SDK officiel autorisé ; elle ne fait pas
partie des fichiers redistribués. Un essai de DemoC ou de ce négatif original
ne remplace pas le contrôle d’un PFL tiers x64 publié prévu en I.1.

Après exécution, conserver capture des commandes, fichiers avant/après,
empreintes, comparaison chiffrée et limites rencontrées. Cocher la validation
croisée seulement lorsque ces résultats sont disponibles. Le pack de cinq PFL
de la partie IV aura ensuite son propre rapport par extension.
