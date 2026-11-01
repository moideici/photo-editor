# Fixture Photoshop 8BF

eightbf_upstream.c provient sans modification de tests/fixtures/testfilter.c,
https://github.com/distortionary-labs/8bf-host, révision
2a52154f160bb25c9ab2a77c29c7ddaccf0b8258, licence MIT choisie.
Copyright 2026 Durable Programming, LLC ; licence conservée à côté du C.

eightbf_fixture.c ajoute seulement des exports de crash, délai et dialogue
natif qui se ferme automatiquement pour les tests de Prisme.
work/build-eightbf-fixtures.py compile une DLL PE x64 avec treize ressources
PiPL et treize filtres distincts. Aucun SDK Adobe, fichier PhotoFiltre ou
filtre propriétaire n’est nécessaire à cette fixture.
Elle n’est pas distribuée dans le bundle de l’application.
