# FRAME Screenshot Studio

Par **zemci**. Studio de captures Geometry Dash écrit en Rust, avec galerie et éditeur plein écran. Un seul mod contient les deux langues ; l’anglais est choisi au premier lancement.

**Windows x64 · Geometry Dash 2.2081 · Geode 5.10.1 / version 5.x compatible**

[Télécharger](https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD/releases) · [English guide](README.md)

## Installation et mise à jour

Ferme GD puis place **zemci.frame.geode** dans **Geometry Dash/geode/mods**. Pour reprendre une ancienne installation FRAME, extrais le ZIP complet et lance **source/install.ps1** avec PowerShell. L’installateur reconnaît les anciens paquets, importe les captures et réglages manquants, sauvegarde les anciens paquets et n’en garde qu’un installé. Les données originales et les fichiers déjà présents dans la nouvelle destination sont conservés.

Indique `-GamePath` si ton jeu est ailleurs. `-PreviousDataDirectory` permet d’importer un dossier de données sauvegardé. Un remplacement manuel du paquet n’importe pas automatiquement les données d’un autre identifiant de mod.

## Utilisation

**F6** ouvre/ferme FRAME. **F8** prend une capture. Ces touches et leurs combinaisons se règlent dans les raccourcis. Choisis **Français / English** dans **Settings > Interface and files**. Le changement est immédiat et mémorisé. Mets GD en pause avant de modifier une image.

- Texte : sélectionne l’outil, clique dans l’image et écris. Entrée valide, Maj+Entrée ajoute une ligne, Échap annule. Double-clique un texte existant pour le corriger.
- Cadres et formes : maintiens la souris et relâche pour valider. L’outil reste actif pour en créer plusieurs. Recliquer l’outil actif revient à la sélection.
- Déplacement : sélectionne une annotation et glisse-la. Les coins la redimensionnent ; Maj contraint le carré/cercle. Le panneau des propriétés ajuste couleurs, dimensions, remplissage et épaisseur.
- Main, zoom, recadrage libre ou par ratio, coordonnées précises, calques et historique. Ctrl+Z/Y annule/rétablit ; Ctrl+D duplique ; Suppr retire ; les flèches déplacent.
- Galerie avec favoris, corbeille récupérable et renommage directement dans l’interface.

**Copier** prend l’image modifiée. **Enregistrer sous** choisit un fichier PNG/JPEG ; **Exporter** crée une copie dans le dossier exports. Les originaux restent séparés et protégés.

## Réglages

Flash, son, aperçu, copie automatique, seuil/plage à la mort, cooldown, pratique, raccourcis, échelle d’interface et application locale facultative pour ouvrir les exports. La capture à la mort et la copie automatique sont désactivées par défaut.

La capture automatique attend la mort confirmée par GD, puis lit une seule image au prochain rendu. Aucune image n’est lue en continu pendant la partie. Un reset avant le rendu annule la demande. Les effets de mort peuvent apparaître ; une lecture ponctuelle peut ralentir brièvement une très grande résolution.

Sources, compilation, licences et limites de validation : [guide anglais](README.md).
