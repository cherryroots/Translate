# Translate

Chinese to English chat translation for WoW: Forever and retail World of Warcraft, in two parts:

- **`addon/Translate`**: the in-game addon. One folder loads in both clients (`## Interface: 16001, 120105`).
- **`companion`**: a small Rust app that runs next to the game, translates through any OpenAI-compatible API, and draws the English over your chat box.

WoW addons cannot reach the internet, and WoW only writes its chat log at logout, so the addon hands new lines to the companion through the screen and the clipboard. Nothing presses keys for you.

## How it works

1. **Automatic:** when a Chinese line arrives, the addon draws it as a small strip of colored blocks in the top-left corner of the screen (256 x 32 pixels). The companion finds the strip on screen, decodes it, translates the line and shows the English in a see-through, click-through overlay that you place over your chat frame.
2. **Click to translate, inside the game:** Chinese lines get a blue `[T]` link. Clicking it opens a small window with the line already highlighted. Press **Ctrl+C**, wait for the overlay to say the line is ready, then click the lower box and press **Ctrl+V**. The English is printed in chat under an `[EN]` tag.
3. **Replying:** type English in the companion's reply box and press Enter. The Chinese is put on your clipboard, ready to paste into WoW chat.

Reading the strip is screen capture: it only reads pixels and never sends input to the game, but Blizzard's terms do not clearly allow or forbid it. Turn the strip off with `/tr strip` to rely on the click-to-translate route alone.

## Install the addon

Copy `addon/Translate` to the client you play:

```
Forever beta: World of Warcraft\_classic_beta_\Interface\AddOns\Translate\
Retail:       World of Warcraft\_retail_\Interface\AddOns\Translate\
```

Restart the client if the addon does not appear in the list. If retail lists it as out of date, the retail patch has moved past `120105`: check the number with `/dump select(4, GetBuildInfo())` and update the `## Interface:` line. In game:

| Command | Does |
| --- | --- |
| `/tr` | Open the window on the latest Chinese line (also use the `<` `>` buttons) |
| `/tr strip` | Toggle the pixel strip the companion reads |
| `/tr links` | Toggle the `[T]` links |
| `/tr test` | Say a Chinese test line in `/say` to check the strip and overlay |
| `/tr probe` | Report which client features the addon found |

## Run the companion

Download a build from the **Actions** tab (open the latest *Build* run and grab `translate-companion-<your OS>` under Artifacts), from a tagged release, or build it yourself with `cargo build --release` inside `companion/` (Linux also needs the PipeWire, X11 and Wayland development packages listed in `.github/workflows/build.yml`).

On macOS and Linux, browsers drop the execute permission, so run `chmod +x translate-companion-*` once. The macOS build is also unsigned: run `xattr -d com.apple.quarantine translate-companion-macos-*` or right-click it and choose Open. macOS will also ask for Screen Recording permission so the companion can read the strip.

On first start it writes `config.json` next to the program. Open **Settings** in the app to set the **API base URL / key / model**: any OpenAI-compatible endpoint. A bare address like `http://localhost:1234` gets `/v1` added.

- **Default:** Tencent's translation model [HY-MT1.5-7B](https://huggingface.co/tencent/HY-MT1.5-7B-GGUF) on a local [Ollama](https://ollama.com) server: run `ollama pull hf.co/tencent/HY-MT1.5-7B-GGUF:Q4_K_M` once (4.6 GB). It is free and works offline.
- **LM Studio:** load the same GGUF, start the server, and use `http://localhost:1234/v1` with the model name LM Studio shows.
- **OpenAI:** `https://api.openai.com/v1` with your key and a model such as `gpt-4o-mini`.

When the model name contains `hy-mt`, the companion switches to HY-MT's own prompt and sampling settings from its model card, and passes the WoW slang found in each line (缺奶, 坦克, 副本...) as terminology hints. Other models get a system prompt that explains the slang instead.

**Test model** sends a fixed Chinese line to the configured model and shows the answer and how long it took. The window also says whether it has found the pixel strip.

Run WoW in **windowed** or **windowed (fullscreen)** mode; overlays cannot draw over exclusive fullscreen, and screen capture may see a black screen there. Click **Unlock overlay** to drag it over your chat frame, then **Lock** it so clicks pass through to the game. Keep the overlay off the top-left corner so it does not cover the strip.

To debug the strip, take a screenshot and run `translate-companion --decode screenshot.png`.

## Known limits

- Chat text inside instances may be a secret value under Midnight rules. The addon cannot read those lines, so they are not translated.
- Each strip frame holds 376 bytes, which fits any WoW chat message (255 bytes at most) plus the channel and name. Longer text would be cut at a character boundary.
- The strip assumes the UI is drawn at the screen's physical resolution. If the companion never finds it, send a screenshot through `--decode`.
