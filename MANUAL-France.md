# open-easy-web FAQ d'auto-hébergement (configuration du compte et 2FA)

📖 Autres langues : [日本語](MANUAL.md) / [English](MANUAL-English.md) /
[中文](MANUAL-Chinese.md) / [한국어](MANUAL-Korea.md) /
[Español](MANUAL-Spain.md) / [Français](MANUAL-France.md) /
[Deutsch](MANUAL-Germany.md) / [Italiano](MANUAL-Italy.md) /
[Русский](MANUAL-Russia.md) / [العربية](MANUAL-Arabic.md) /
[Português](MANUAL-Portugal.md) / [Nederlands](MANUAL-Netherlands.md) /
[Türkçe](MANUAL-Turkey.md) / [Polski](MANUAL-Poland.md) /
[Tiếng Việt](MANUAL-Vietnam.md) / [ไทย](MANUAL-Thailand.md) /
[Bahasa Indonesia](MANUAL-Indonesia.md) / [हिन्दी](MANUAL-India.md)

---

## Q1. Si je télécharge ceci et l'exécute sur mon propre VPS, PC, téléphone ou tablette, puis-je enregistrer ma propre adresse e-mail et mon propre numéro de téléphone ?

**Oui.** Il n'existe pas de formulaire d'inscription en libre-service dans le navigateur (l'inscription publique a été intentionnellement désactivée le 2026-07-15 pour des raisons de sécurité). À la place, vous définissez **votre propre** adresse e-mail et numéro de téléphone comme unique compte de connexion via des **variables d'environnement** au démarrage du serveur.

| Variable d'environnement | Obligatoire/Facultatif | Signification |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Obligatoire | Votre propre adresse e-mail |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Facultatif | Votre propre numéro de téléphone |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Facultatif | Une adresse e-mail de secours |

Si vous ne définissez pas de numéro de téléphone, une adresse e-mail de secours est requise (au moins l'un des deux doit être défini).

**Comment le configurer selon la plateforme :**
- **Windows / Linux (VPS, etc.)** : définissez-la en variable d'environnement lors de l'installation, ou dans le fichier de service systemd.
- **Android** : saisissez votre adresse e-mail dans l'écran « Configuration du compte fixe » de l'application (l'application refuse de démarrer si cela n'est pas défini — une mesure de sécurité délibérée).

En résumé : votre propre instance auto-hébergée utilise exactement le même mécanisme que l'instance de production (easy-web.tokyo), qui fonctionne elle-même avec l'adresse du propriétaire.

## Q2. Si je n'ai qu'un téléphone basique (non smartphone), puis-je confirmer l'authentification à deux facteurs (2FA) sur mon PC ?

**Oui.** L'écran de configuration 2FA (TOTP via application d'authentification) n'affiche pas d'image de code QR destinée à être scannée par l'appareil photo d'un smartphone — il affiche directement la **chaîne secrète en texte brut**.

Cette chaîne fonctionne avec n'importe quelle application TOTP permettant de saisir un secret manuellement — pas seulement les authentificateurs de smartphone. Si vous n'avez qu'un téléphone basique, vous avez deux options :

1. Utilisez plutôt l'**OTP par e-mail** (l'option la plus simple si votre téléphone basique peut recevoir des e-mails de l'opérateur).
2. Saisissez le « secret » affiché lors de la configuration 2FA dans une **application d'authentification pour PC** (par exemple WinAuth, ou une extension de navigateur d'authentification), puis lisez le code à 6 chiffres affiché sur l'écran de votre PC lors de la connexion.

Les deux méthodes fonctionnent immédiatement sans configuration particulière.
