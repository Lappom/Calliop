# Benchmarks publics

Métriques STT reproductibles pour Calliop v0.1.0, alignées sur la config cible du [PLAN.md](../PLAN.md) : **16 Go RAM, CPU/iGPU sans GPU dédiée**.

## Méthodologie

| Paramètre | Valeur |
|-----------|--------|
| Corpus | 5 phrases françaises synthétiques (SAPI) — [`benchmarks/corpus/fr.json`](../benchmarks/corpus/fr.json) |
| Modèle STT | Whisper `ggml-small.bin` |
| Audio | PCM 16 kHz mono |
| Métrique WER | Distance d’édition au niveau mot, normalisation ponctuation/casse |
| Latence | Temps wall-clock `WhisperEngine::transcribe` par échantillon |

### Reproduire

```powershell
# Générer le corpus audio (Windows + ffmpeg)
powershell -ExecutionPolicy Bypass -File scripts/generate-benchmark-corpus.ps1

# Exécuter le benchmark (télécharge le modèle au 1er run)
cd src-tauri
cargo run --release --bin benchmark-stt -- ../benchmarks/corpus/fr.json --cpu
```

Le rapport JSON est écrit dans `benchmarks/results/v0.1.0.json`.

## Résultats v0.1.0

Corpus synthétique SAPI (5 phrases, ~4 s chacune), Whisper `small`, CPU, Windows.

| Métrique | Valeur | Cible PLAN |
|----------|--------|------------|
| WER moyen | **6,0 %** | < 15 % MVP |
| Latence STT moyenne (corpus ~4 s) | **10,4 s** | < 2 s (phrase ~10 mots, pipeline chaud) |

Machine de référence : AMD Ryzen 7 9700X, 16 Go RAM, CPU uniquement (build sans GPU). La latence CLI inclut l’inférence complète par échantillon ; en dictée réelle, le moteur reste chargé et les segments courts sont plus rapides (voir onglet Insight).

Détail par échantillon : [`benchmarks/results/v0.1.0.json`](../benchmarks/results/v0.1.0.json). Régénérer après modification du corpus avec `benchmark-stt`.

Les benchmarks LLM (auto-edit) et latence bout-en-bout (hotkey → injection) sont suivis dans l’onglet **Insight** de l’application.

## CI

Les tests unitaires WER (`src-tauri/src/stt/wer.rs`) s’exécutent sans réseau ni modèle lourd. Le bench complet reste manuel ou sur runner dédié.
