# Provenance du cœur 8BF

Source : https://github.com/distortionary-labs/8bf-host
Révision : 2a52154f160bb25c9ab2a77c29c7ddaccf0b8258
Auteur : Durable Programming, LLC. Licence MIT choisie parmi MIT / Apache-2.0.

Seul src/host est repris, avec les adaptations listées ci-dessous. Les interfaces CLI/HTTP,
les photos et les autres ressources ne sont pas intégrées. Cargo.toml et
src/lib.rs sont adaptés pour exposer le cœur comme dépendance locale de Prisme.
Les références à eightbf_host sont conservées. Les licences originales sont
incluses. Ce composant ne provient pas de PhotoFiltre.

L'intégration Prisme utilise exclusivement ce cœur dans son processus hôte
isolé. Les limites d'interopérabilité seront vérifiées par les fixtures C et
par un filtre 8BF indépendant, sans assimiler une simple découverte à une
exécution réussie.

## Adaptation Prisme du 25 octobre 2026

session.rs : lorsqu’une requête possède un descripteur de script et ne demande
pas de dialogue, ne pas appeler le sélecteur Parameters pour la seule raison
que le handle de paramètres est nul. Le sélecteur Start initialise les handles
du plugin puis lit son descripteur. Filter Foundry utilise Parameters pour
forcer son dialogue ; ses wrappers de handles ont aussi besoin de cette
initialisation avant de lire des paramètres dans un nouveau processus.
La preuve applicative doit couvrir un filtre tiers réellement exécuté avec
ses formules et le rejeu de son descripteur dans un autre processus.
