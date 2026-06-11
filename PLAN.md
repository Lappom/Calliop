# Calliop — Plan de développement

> Clone open source de [Wispr Flow](https://wisprflow.ai/) : dictée vocale universelle, 100 % locale, développée en vibe coding avec Cursor.

---

## Vision produit

Application desktop qui permet de **dicter dans n'importe quelle application** avec :

- Raccourci global (push-to-talk ou toggle)
- Transcription locale multilingue
- Nettoyage IA du texte (suppression des fillers, ponctuation, reformulation légère)
- Dictionnaire personnel et snippets vocaux
- Adaptation du ton selon l'application active
- Synchronisation des réglages (local uniquement — pas de cloud obligatoire)
- Historique des dictées

**Contrainte absolue :** aucune dépendance à un service cloud pour le fonctionnement core. Tout tourne en local.

---

## Parité fonctionnelle avec Wispr Flow

| Feature Wispr Flow | Priorité | Phase |
|---|---|---|
| Dictée dans toute app (hotkey global) | P0 | 1 |
| Transcription speech-to-text | P0 | 1 |
| Push-to-talk | P0 | 1 |
| Auto-edits IA (texte poli) | P0 | 3 |
| Overlay / indicateur visuel | P1 | 2 |
| Toggle mode (sans maintenir la touche) | P1 | 2 |
| Dictionnaire personnel | P1 | 3 |
| Snippets vocaux | P1 | 3 |
| Détection / switch multilingue | P1 | 3 |
| Contexte par application (ton adapté) | P2 | 3 |
| Historique + recherche | P2 | 3 |
| System tray + démarrage auto | P1 | 2 |
| Paramètres complets (modèles, hotkeys, langues) | P1 | 4 |
| Installeurs multi-plateforme | P1 | 4 |
| Onboarding premier lancement | P2 | 4 |

---

## Stack technique recommandée

| Composant | Technologie | Rôle |
|---|---|---|
| Shell desktop | **Tauri 2** (Rust + React/TS) | App légère, tray, hotkeys, permissions OS |
| Capture audio | `cpal` (Rust) | Micro cross-platform, faible latence |
| VAD | **Silero VAD** (ONNX) | Détection voix / silence |
| STT | **whisper.cpp** (`whisper-rs`) ou **faster-whisper** | Transcription locale, 100+ langues |
| Post-traitement IA | **llama.cpp** (Qwen 2.5 3B / Llama 3.2 3B) | Auto-edits, snippets, ton par app |
| Injection texte | `enigo` + clipboard fallback | Coller dans l'app active |
| Stockage | SQLite (`rusqlite`) | Config, dictionnaire, snippets, historique |
| UI | React + Tailwind (webview Tauri) | Overlay + fenêtre réglages |

### Alternatives à évaluer

- **Electron** au lieu de Tauri : plus lourd, mais écosystème plus mature pour certaines APIs.
- **Python backend** (faster-whisper + PyQt) : prototypage rapide, packaging plus difficile.
- **Fork d'un projet existant** (VoiceInk, Whispering, Handy) : gain de temps sur l'injection texte.

---

## Architecture cible

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                      │
│  Overlay │ Settings │ Onboarding │ History │ Tray UI    │
└────────────────────────┬────────────────────────────────┘
                         │ Tauri IPC
┌────────────────────────▼────────────────────────────────┐
│                   Core (Rust)                            │
│  HotkeyManager │ AudioCapture │ VAD │ PipelineOrchestrator│
└───┬──────────────┬──────────────┬──────────────┬────────┘
    │              │              │              │
    ▼              ▼              ▼              ▼
 WhisperEngine  LLMEngine   TextInjector   SQLiteStore
 (STT local)   (auto-edits)  (paste/type)  (config/data)
```

### Pipeline audio (happy path)

1. Utilisateur maintient le hotkey (ou toggle ON)
2. `AudioCapture` enregistre PCM 16 kHz mono
3. `VAD` filtre les silences (optionnel en streaming)
4. Relâchement hotkey → buffer audio complet
5. `WhisperEngine` transcrit (+ `initial_prompt` dictionnaire)
6. `LLMEngine` nettoie / applique snippets / adapte le ton (si activé)
7. `TextInjector` insère le texte dans l'app au premier plan
8. `SQLiteStore` persiste l'entrée dans l'historique

---

## Phases de développement

### Phase 0 — Fondations (Jour 1)

**Objectif :** repo prêt pour le vibe coding.

- [x] Scaffold Tauri 2 + React + TypeScript
- [x] Init Git, choix licence, README, CONTRIBUTING
- [x] CI GitHub Actions (build Win/macOS/Linux, lint, tests)
- [x] Règles Cursor (`.cursor/rules/`) :
  - architecture pipeline audio
  - contrainte 100 % offline
  - conventions Rust / TS
  - stratégie de test par module
- [x] Structure dossiers :
  ```
  src-tauri/src/
    audio/       # capture, VAD
    stt/         # whisper bindings
    llm/         # post-processing local
    inject/      # text injection
    hotkey/      # global shortcuts
    store/       # SQLite
    pipeline/    # orchestration
  src/
    components/  # overlay, settings, onboarding
    hooks/
    lib/
  ```

**Critère de done :** `cargo tauri dev` lance une fenêtre vide + tray icon.

---

### Phase 1 — MVP « ça marche » (Semaines 1–2)

**Objectif :** dicter une phrase dans Notepad via hotkey.

- [x] Hotkey global **Alt + Espace** (toggle)
- [x] Capture micro → buffer PCM
- [x] Transcription Whisper à la fin de la dictée (toggle OFF)
- [x] Injection texte (clipboard + Ctrl+V avec restauration)
- [x] Téléchargement modèle Whisper au 1er lancement
- [x] CLI de test par module (`cargo run --bin test-audio`, etc.)

**Critère de done :** dicter « Bonjour, ceci est un test » dans Notepad, Word, navigateur.

**Risque #1 :** injection texte — implémenter fallback clipboard dès le début.

---

### Phase 2 — Expérience temps réel (Semaines 3–4)

**Objectif :** usage fluide au quotidien.

- [x] Silero VAD (ignore silences)
- [x] Mode toggle (en plus du push-to-talk)
- [x] Transcription streaming par chunks
- [x] Overlay flottant (waveform, états écoute/transcription)
- [x] Icône system tray + menu contextuel
- [x] Démarrage automatique avec l'OS
- [x] Indicateur latence (debug)

**Critère de done :** dictée fluide sans latence perçue > 500 ms (streaming).

---

### Phase 3 — Features « Wispr » (Semaines 5–7)

**Objectif :** différenciation vs simple Whisper GUI.

#### 3a — Auto-edits IA
- [x] Intégration llama.cpp (modèle 3B quantifié)
- [x] Prompt de nettoyage (fillers, ponctuation, reformulation)
- [x] Toggle on/off + mode « verbatim » (sans LLM)
- [x] Exécution async (ne bloque pas l'injection si LLM lent)

#### 3b — Dictionnaire personnel
- [x] CRUD mots / noms propres dans Settings
- [x] Injection dans `initial_prompt` Whisper
- [x] Apprentissage auto des corrections utilisateur
- [x] Notifications sur les corrections utilisateur

#### 3c — Snippets vocaux
- [x] Définition trigger → texte complet
- [x] Matching dans post-processing LLM
- [x] Import / export JSON

#### 3d — Contexte par application
- [ ] Détection fenêtre active (titre, bundle ID / exe)
- [ ] Profils de ton : casual (Slack), formel (mail), technique (IDE/terminal)
- [ ] Mapping configurable par app

#### 3e — Multilingue
- [ ] Auto-détection langue (Whisper)
- [ ] Langue par défaut configurable
- [ ] Switch mid-dictée

#### 3f — Historique
- [ ] Toutes les dictées en SQLite
- [ ] Recherche full-text
- [ ] Re-copie / réinjection

**Critère de done :** dicter un message Slack poli, un commit message technique, un snippet « mon calendrier ».

---

### Phase 3.5 — Optimisation des performances

**Objectif :** optimiser les performances du pipeline.

- [ ] Optimiser le pipeline audio
- [ ] Optimiser le pipeline de transcription
- [ ] Optimiser le pipeline de post-traitement
- [ ] Optimiser le pipeline de notification
- [ ] Optimiser le pipeline de démarrage automatique
- [ ] Optimiser le pipeline d'apprentissage automatique des corrections utilisateur
- [ ] Optimiser le pipeline de dictionnaire personnel

---

### Phase 4 — Polish & distribution (Semaines 8–10)

**Objectif :** produit installable par un non-développeur.

- [ ] Fenêtre Settings complète
- [ ] Gestion modèles (small / medium / large, CPU vs GPU)
- [ ] Onboarding (permissions micro + accessibilité, test dictée)
- [ ] Installeurs : MSI/NSIS (Win), DMG (macOS), AppImage/deb (Linux)
- [ ] Auto-update Tauri (optionnel, désactivable)
- [ ] Benchmarks publics (latence, WER)
- [ ] Documentation utilisateur

**Critère de done :** installateur Windows testé sur machine vierge, dictée fonctionnelle en < 5 min.

---

### Phase 5 — Open source & communauté

- [ ] Releases GitHub avec binaires + changelog
- [ ] Roadmap publique (GitHub Projects)
- [ ] Labels `good first issue`
- [ ] Discord / GitHub Discussions
- [ ] Politique de sécurité (SECURITY.md)
- [ ] Comparatif honnête vs Wispr Flow (features, perf, limites)

---

## Workflow vibe coding (Cursor)

### Par session

1. **Une feature = une branche = une session**
2. Surligner une section de ce plan → **`/implement`** dans Cursor (ou demander un plan manuel → valider → implémenter)
3. Boucle : code → test → coller logs/erreurs → itérer
4. Commit après chaque étape verte
5. Mettre à jour `.cursor/rules/` après chaque décision d'architecture

### Prompts types

```
Phase 1 — Implémente le module AudioCapture avec cpal.
- PCM 16 kHz mono
- Start/stop via commande Tauri
- Test CLI : cargo run --bin test-audio -- record 3s output.wav
```

```
Phase 3 — Ajoute le post-processing LLM.
- Modèle Qwen 2.5 3B Q4 via llama.cpp
- Prompt : nettoyer fillers, ponctuation, garder le sens
- Toggle settings.auto_edit
- Fallback : retourner transcription brute si LLM indisponible
```

### Anti-patterns à éviter

- ❌ « Fais toute l'application » en un prompt
- ❌ Mélanger plusieurs phases dans une session
- ❌ Ignorer les tests CLI des modules Rust isolés
- ❌ Appels réseau « temporaires » qui deviennent permanents

---

## Risques & mitigations

| Risque | Impact | Mitigation |
|---|---|---|
| Injection texte fragile (Electron apps, jeux, champs sécurisés) | Haut | Clipboard-paste + restauration ; tester sur 10+ apps dès Phase 1 |
| Permissions macOS (Accessibility) | Haut | Demander tôt ; documenter ; fallback presse-papier |
| Latence LLM sur CPU modeste | Moyen | Auto-edit optionnel ; modèle 3B quantifié ; async |
| Taille binaires + modèles | Moyen | Téléchargement modèles au 1er lancement ; choix small/medium |
| Notarisation Apple (99 $/an) | Moyen | Phase 4 ; distribution unsigned en dev |
| Qualité STT accents / jargon | Moyen | Dictionnaire perso + fine-tuning prompt |

---

## Projets open source à étudier

| Projet | Plateforme | Intérêt |
|---|---|---|
| VoiceInk | macOS | Injection texte, UX |
| Whispering | Cross | Pipeline Whisper |
| Handy | Windows | Hotkeys, tray |
| whisper.cpp | Cross | Bindings STT |
| OpenWhispr | Cross | Architecture de référence |

**Décision actée :** repo **100 % custom** (Calliop from scratch). VoiceInk / Whispering / Handy servent de référence, pas de fork.

---

## Métriques de succès

| Métrique | Cible MVP | Cible v1.0 |
|---|---|---|
| Latence STT (phrase 10 mots) | < 2 s | < 1 s (streaming) |
| Latence auto-edit | N/A | < 3 s (async OK) |
| WER français | < 15 % | < 10 % |
| Apps compatibles injection | 5+ | 20+ |
| Taille installateur | < 50 Mo | < 30 Mo (+ modèles séparés) |
| RAM idle | < 200 Mo | < 150 Mo |

---

## Prochaines actions immédiates

1. ~~Répondre aux questions de cadrage~~ ✅ **Complet**
2. ~~Lancer Phase 0~~ — scaffold Tauri 2 + règles Cursor + CI ✅ **Complet**
3. **Premier milestone** — MVP Notepad (Phase 1) : Alt+Espace → Whisper → injection

---

## Décisions prises

| Question | Décision |
|---|---|
| Nom du produit | **Calliop** |
| Framework desktop | **Tauri 2** (Rust + React/TS) |
| Plateformes v1 | **Windows uniquement** — macOS/Linux en v2+ |
| Licence | **AGPL-3.0** |
| Langue UI | **Français uniquement** (v1) |
| Langue STT / tests | **Français d'abord** — multilingue en Phase 3+ |
| Approche | **100 % custom** — repo Calliop from scratch ; libs éprouvées (whisper.cpp, cpal) sans fork |
| GPU | **Optionnel** — CUDA/Vulkan sur Windows si disponible, **fallback CPU** obligatoire |
| Machine cible minimale | **16 Go RAM, CPU/iGPU uniquement** — perf acceptable sans GPU dédiée |
| Hotkey par défaut | **Alt + Espace** (toggle) |
| Hébergement modèles | **GitHub Releases + Hugging Face** (fallback) |
| Code signing Windows | **Unsigned OK** pour v1 |
| Point de départ code | **100 % custom** — repo Calliop from scratch (cf. Approche) |
| Auto-edits LLM | **Phase 3** — MVP = transcription brute fiable |
| Modèles | **Téléchargement au 1er lancement** — installateur léger |

### Conséquences sur le plan

- Phase 1 simplifiée : pas de LLM, focus hotkey → Whisper → injection.
- Phase 0 : prévoir une couche `InferenceBackend` (CPU par défaut, CUDA/Vulkan optionnels) même si seul Windows est testé en v1.
- Installeur Windows : ~15–25 Mo ; modèles Whisper (~150 Mo–1,5 Go) + LLM Phase 3 (~2 Go) au premier run.
- macOS/Linux reportés : ne pas bloquer l'architecture, mais ne pas investir en notarisation / AppImage avant v1 Windows stable.
- MVP testé sur machine **16 Go RAM sans GPU** ; Whisper `small` ou `base` recommandé par défaut, calibré **français**.
- Hotkey **Alt+Espace** en toggle (pas de push-to-talk obligatoire en Phase 1).
- Multilingue (100+ langues) : Phase 3, une fois le pipeline FR stable et benchmarké.
- **Téléchargement modèles** : GitHub Releases en source primaire, Hugging Face en fallback automatique.
- **Installeur v1 unsigned** : pas de certificat code signing ; SmartScreen Windows possible — documenter dans README.
- **Benchmarks** : toutes les métriques de perf validées sur config **16 Go RAM / CPU-iGPU sans GPU dédiée**.
