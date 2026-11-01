# Exemple PFL Unicode x64 — MIT

`prisme_pfl.h` décrit l’ABI PFL, distincte de l’API DLL Prisme.
Le contrat et les limites de l’hôte de développement sont dans ../../PFL.md.
Les tailles/offsets sont documentés dans ../../PFL_ABI.md.

`examples/negative.c` est un filtre original de niveau 2. Il utilise les exports
PFL requis et les DBmp de l’hôte, sans dépendance au code ni aux ressources de
PhotoFiltre. `examples/negative.pfl` est sa compilation Windows x64.

Depuis un terminal développeur MSVC x64, dans examples :

```bat
cl /nologo /LD /O2 /utf-8 negative.c /link /OUT:negative.pfl
```

Dans Prisme, ouvrir une image bitmap puis charger ce .pfl depuis le catalogue
PFL. Le filtre inverse le RGB de la sélection et garde l’alpha. Il est exécuté
dans un processus séparé et peut être annulé.

Ce premier exemple ne constitue pas le pack d’enrichissement prévu en partie IV
du plan. La comparaison du même fichier dans PhotoFiltre reste à réaliser.
