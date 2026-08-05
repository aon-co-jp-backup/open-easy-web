# open-easy-web FAQ de auto-hospedagem (configuração de conta e 2FA)

📖 Outros idiomas: [日本語](manual-JAPAN.md) / [English](manual-ENGLISH.md) /
[中文](manual-CHINA.md) / [한국어](manual-KOREA.md) /
[Español](manual-SPAIN.md) / [Français](manual-FRANCE.md) /
[Deutsch](manual-GERMANY.md) / [Italiano](manual-ITALY.md) /
[Русский](manual-RUSSIA.md) / [العربية](manual-ARABIA.md) /
[Português](manual-PORTUGAL.md) / [Nederlands](manual-NETHERLANDS.md) /
[Türkçe](manual-TURKEY.md) / [Polski](manual-POLAND.md) /
[Tiếng Việt](manual-VIETNAM.md) / [ไทย](manual-THAILAND.md) /
[Bahasa Indonesia](manual-INDONESIA.md) / [हिन्दी](manual-INDIA.md) /
[فارسی](manual-IRAN(PERUSHA).md)

---

## P1. Se eu baixar isto e executar no meu próprio VPS, PC, telemóvel ou tablet, posso registar o meu próprio endereço de email e número de telefone?

**Sim, pode.** Não existe um formulário de registo autoatendido no navegador (o registo público foi intencionalmente desativado em 2026-07-15 por motivos de segurança). Em vez disso, você define o **seu próprio** endereço de email e número de telefone como a única conta de login através de **variáveis de ambiente** no arranque do servidor.

| Variável de ambiente | Obrigatório/Opcional | Significado |
|---|---|---|
| `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL` | Obrigatório | O seu próprio endereço de email |
| `OPEN_EASYWEB_FIXED_ACCOUNT_PHONE` | Opcional | O seu próprio número de telefone |
| `OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL` | Opcional | Um endereço de email de reserva |

Se não definir um número de telefone, é necessário um email de reserva (pelo menos um dos dois deve ser definido).

**Como configurar por plataforma:**
- **Windows / Linux (VPS, etc.)**: defina como variável de ambiente na instalação, ou no ficheiro de serviço systemd.
- **Android**: introduza o seu endereço de email no ecrã "Configuração de conta fixa" da aplicação (a aplicação recusa-se a iniciar se isto não estiver definido — uma medida de segurança deliberada).

Em resumo: a sua própria instância auto-hospedada usa exatamente o mesmo mecanismo que a instância de produção (easy-web.tokyo), que por sua vez funciona com o endereço do próprio proprietário.

## P2. Se eu só tiver um telemóvel básico (não smartphone), posso confirmar a autenticação de dois fatores (2FA) no meu PC?

**Sim, pode.** O ecrã de configuração de 2FA (TOTP via aplicação autenticadora) não mostra uma imagem de código QR para ser digitalizada pela câmara do smartphone — mostra diretamente a **cadeia secreta em texto simples**.

Essa cadeia funciona com qualquer aplicação TOTP que permita introduzir um segredo manualmente — não apenas com autenticadores de smartphone. Se tiver apenas um telemóvel básico, tem duas opções:

1. Use **OTP por email** (a opção mais simples se o seu telemóvel básico conseguir receber emails da operadora).
2. Introduza manualmente o "segredo" mostrado durante a configuração de 2FA numa **aplicação autenticadora de PC** (por exemplo, WinAuth ou uma extensão de navegador), e depois leia o código de 6 dígitos no ecrã do PC ao iniciar sessão.

Ambos os métodos funcionam de imediato sem configuração especial.
