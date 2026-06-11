# Dépannage

## L’installateur est bloqué par SmartScreen

Les binaires v1 ne sont pas signés. Utilisez **Informations complémentaires → Exécuter quand même**. Pour un déploiement entreprise, prévoyez une exception ou un certificat de code signing.

## L’app ne démarre pas (écran noir)

Installez ou réparez [WebView2 Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/).

## Le micro ne fonctionne pas

1. Vérifiez les autorisations Windows : **Paramètres → Confidentialité → Microphone**.
2. Dans Calliop, refaites le test micro depuis l’onboarding.
3. Fermez les autres apps qui monopolisent le micro.

## La dictée ne s’injecte pas

1. Placez le curseur dans un champ texte (Notepad pour tester).
2. Vérifiez que le raccourci global n’est pas en conflit avec une autre application.
3. Certaines apps sécurisées (jeux, champs mot de passe) peuvent bloquer l’injection — Calliop utilise le presse-papiers en secours.

## Téléchargement du modèle bloqué

- Vérifiez la connexion internet et un éventuel proxy.
- Les modèles sont hébergés sur Hugging Face (fallback si GitHub Releases indisponible).
- Espace disque : prévoir au moins 2 Go pour Whisper + LLM.

## Latence élevée

1. Utilisez Whisper `small` et LLM Qwen3 0.6B.
2. Paramètres → Avancé → **CPU uniquement** si le GPU Vulkan pose problème.
3. Fermez les applications gourmandes en RAM (cible : 16 Go).

## Mises à jour

Si les mises à jour automatiques échouent :

1. Vérifiez que l’option est activée dans Paramètres → Avancé.
2. Téléchargez manuellement depuis [GitHub Releases](https://github.com/Lappom/Calliop/releases).

## Logs

Lancez Calliop depuis un terminal pour voir les messages `stderr` :

```powershell
& "$env:LOCALAPPDATA\Programs\Calliop\Calliop.exe"
```

En développement : `pnpm tauri:dev`.

## Données utilisateur

Configuration et historique : `%APPDATA%\com.calliop.app\`  
Modèles : `%APPDATA%\com.calliop.app\models\`
