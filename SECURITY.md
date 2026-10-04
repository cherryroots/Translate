# Security policy

## Supported versions

Only the latest release and the `main` branch get fixes.

## Reporting a vulnerability

Please report security problems privately through GitHub: open the repository's **Security** tab and choose **Report a vulnerability**. Do not open a public issue for them.

Useful things to report include:

- the companion sending chat text or API keys anywhere other than the API base URL you configured;
- crafted chat lines or clipboard text that crash the companion or make it run code;
- the addon tainting secure Blizzard frames.

Your API key is stored in plain text in `config.json` next to the program. Keep that file private, and prefer a local model server if that matters to you.

You should get a reply within a week.
