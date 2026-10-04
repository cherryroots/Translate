# Translate

Chinese to English chat translation for WoW: Forever, in two parts:

- **`addon/Translate`**: the in-game addon (`## Interface: 16001`).
- **`companion`**: a small Rust app that runs next to the game, translates through any OpenAI-compatible API, and draws the English over your chat box.

WoW addons cannot reach the internet, so the two talk through files and the clipboard. Nothing presses keys or reads the screen for you.

## How it works

1. **Automatic:** the addon turns on WoW's chat log (`Logs\WoWChatLog.txt`). The companion follows that file, translates every Chinese line and shows the English in a see-through, click-through overlay that you place over your chat frame.
2. **Click to translate, inside the game:** Chinese lines get a blue `[T]` link. Clicking it opens a small window with the line already highlighted. Press **Ctrl+C**, wait for the overlay to show the English, then click the lower box and press **Ctrl+V**. The English is printed in chat under an `[EN]` tag.
3. **Replying:** type English in the companion's reply box and press Enter. The Chinese is put on your clipboard, ready to paste into WoW chat.

## Install the addon

Copy `addon/Translate` to:

```
World of Warcraft\_classic_beta_\Interface\AddOns\Translate\
```

Restart the client if the addon does not appear in the list. In game:

| Command | Does |
| --- | --- |
| `/tr` | Open the window on the latest Chinese line (also use the `<` `>` buttons) |
| `/tr log` | Toggle the chat log (the companion needs it on) |
| `/tr links` | Toggle the `[T]` links |
| `/tr probe` | Report which client features the addon found |

## Run the companion

Build with `cargo build --release` inside `companion/` (or use a prebuilt `translate-companion.exe`). On first start it writes `config.json` next to the exe. Open **Settings** in the app to set:

- **Chat log file:** defaults to `C:\Program Files (x86)\World of Warcraft\_classic_beta_\Logs\WoWChatLog.txt`.
- **API base URL / key / model:** any OpenAI-compatible endpoint. The default is a local [Ollama](https://ollama.com) server (`http://localhost:11434/v1`, model `qwen2.5:7b`, no key), which is free, offline and good at Chinese. For OpenAI use `https://api.openai.com/v1` with your key and a model such as `gpt-4o-mini`.

Run WoW in **windowed** or **windowed (fullscreen)** mode; overlays cannot draw over exclusive fullscreen. Click **Unlock overlay** to drag it over your chat frame, then **Lock** it so clicks pass through to the game.

## Not yet verified on the Forever beta

Check these with `/tr probe` and a quick test, and report back anything that fails:

- `LoggingChat` exists and the log file updates while you play (if it only updates on logout, the overlay will lag).
- The chat log's line format matches what the companion parses (`date time  [Channel] Name: text`).
- Chat filters (`ChatFrame_AddMessageEventFilter` or `ChatFrameUtil.AddMessageEventFilter`) and `addon:` link clicks through `EventRegistry` work. If not, `/tr` still opens the window.
- Chat text inside instances may be a secret value under Midnight rules. The addon then leaves those lines alone; the overlay still translates them from the log.
