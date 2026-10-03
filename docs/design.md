# FRAME — conception validée

Demande : un mod natif Geode, sans code C++, qui capture Geometry Dash et permet de gérer, zoomer et annoter les captures. L'utilisateur autorise la réalisation autonome et ne sera pas disponible pour répondre.

## Cible
Windows x64, Geometry Dash 2.2081, Geode 5.10.1 stable. Rust avec les bindings communautaires geode-rs et le rendu egui intégré au moteur Cocos. La compatibilité doit être vérifiée contre l'installation locale. Aucune modification automatique de cette installation : livrer un paquet .geode et un installateur local explicite.

## Expérience
F8 : capturer ; F6 : ouvrir/fermer FRAME, raccourcis modifiables. Interface française sombre, accent cyan, caméra originale. Aperçu temporaire en haut à droite, cadre blanc, léger mouvement d'entrée. Flash blanc optionnel et son original doux, réglables. L'image est lue dans le framebuffer avant ces effets et avant l'interface FRAME.

Galerie locale avec vignettes, recherche par niveau, tri récent, favoris, renommage, suppression, ouverture du dossier et export PNG. Les captures enregistrent niveau, pourcentage, tentative et mode pratique quand ces informations sont accessibles. Les fichiers restent accessibles sans le mod.

Éditeur : zoom à la souris, déplacement, dessin libre, flèches, texte, recadrage et export de copie. Annuler et rétablir. L'original est immuable ; les éditions sont enregistrées séparément et reprises à la réouverture.

Auto-capture : activée séparément, mort à partir d'un seuil ou dans une plage inclusive 0–100 %. Filtre pratique, délai entre captures, au maximum une capture par tentative. Capture de la dernière image de jeu au déclenchement de la mort, avant flash/vignette. Le joueur et son partenaire en mode dual ne créent pas deux captures.

## Architecture
Une bibliothèque de logique testable (règles, index, export/édition), un adaptateur natif pour hooks et framebuffer, et une interface egui. Une file de travail encode les PNG hors du fil de rendu. Le fil de rendu conserve uniquement des buffers et vignettes limités. Paramètres et métadonnées JSON, écriture atomique ; erreurs visibles dans FRAME sans panique traversant l'ABI.

## Vérification et livraison
Tests des seuils et plages, répétitions de mort, cooldown, filtre pratique, coordonnées du recadrage, couleur/export des annotations et conservation de l'original. Compilation Rust x64, inspection du paquet et tentative de lancement dans une copie isolée du jeu. Rapporter honnêtement les limites de validation. Assets originaux et sources reproductibles inclus.
