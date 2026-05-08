# 中 ZHLI — Command Line Chinese Trainer

**Version 0.2.0**

A terminal-based flashcard app for learning HSK 1–6 Chinese vocabulary using the SM-2 spaced repetition algorithm. All data is stored locally in SQLite — no account, no internet required for core study.

---

## Features

- HSK levels 1–6 (~5 000 vocabulary cards, four study directions each)
- SM-2 spaced repetition scheduling with per-card ease factor
- Numbered pinyin input (`hao3` → `hǎo`) with partial-credit tone grading
- Custom word decks — a word can belong to multiple decks simultaneously
- HSK words added to a deck remain visible in their HSK level
- Online MDBG dictionary lookup for adding new words
- Edit, suspend/unsuspend, and delete custom words from the search screen
- Stats dashboard: streak, retention, per-level/per-deck progress, weakest words, 7-day activity
- Vim-style navigation (`j`/`k`, `gg`/`G`, `:q`, `:q!`)
- Clipboard yank and browser dictionary lookup during review
- Study mode, content selection, and deck selection persisted across sessions
- RC file for custom database path and session size

---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later (install via `rustup` — recommended)
- A terminal with 256-colour support
- **Linux only:** clipboard support requires a clipboard utility at runtime (see below)

---

## Installation

### Arch Linux

```bash
sudo pacman -S rustup
rustup default stable

# Clipboard dependencies (pick one)
sudo pacman -S xclip          # X11
sudo pacman -S wl-clipboard   # Wayland

# Build-time XCB headers
sudo pacman -S libxcb

git clone https://github.com/radleylewis/zhli.git
cd zhli
cargo build --release
```

### Ubuntu / Debian

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

sudo apt install xclip libxcb1-dev libxcb-render0-dev \
                 libxcb-shape0-dev libxcb-xfixes0-dev
# Wayland: sudo apt install wl-clipboard

git clone https://github.com/radleylewis/zhli.git
cd zhli
cargo build --release
```

### macOS

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

git clone https://github.com/radleylewis/zhli.git
cd zhli
cargo build --release
```

---

## Running

```bash
./target/release/zhli
# or
cargo run --release
```

The app creates its database on first launch and seeds all HSK 1–6 words automatically. Existing databases from earlier versions are upgraded automatically on first run.

**Default data location:**

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/zhli/data.db` |
| macOS    | `~/Library/Application Support/zhli/data.db` |

---

## Configuration

Create `~/.config/zhli/config` (plain text, `key = value`):

```
# Custom database location
db_path = ~/Documents/zhli/data.db

# Cards per review session (default: 50)
session_limit = 100
```

Lines starting with `#` are ignored. Both keys are optional — omit either to use the default.

---

## Usage

### Navigation (global)

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Move cursor |
| `gg` | Jump to top |
| `G` | Jump to bottom |
| `Enter` | Confirm / select |
| `Esc` | Back / cancel |
| `:q` | Back to main menu (or quit from menu) |
| `:q!` | Force quit from anywhere |
| `Ctrl-C` | Quit immediately |

---

### Main Menu

| Option | Description |
|--------|-------------|
| Study / Review Cards | Choose mode and content, then start a session |
| Stats & Report Card | View progress, streaks, and weakest words |
| Add Word to Deck | Create or manage custom decks |
| Search & Browse | Search all words, manage deck membership, edit, suspend |
| About / Rules / Algo | App guide, pinyin rules, SM-2 explanation |
| Clear Custom Words | Delete all custom words and decks |
| Reset All Progress | Reset all SRS state to zero |
| Quit | Exit |

---

### Study Session

**Step 1 — Mode Select:** choose a study direction with `↑`/`↓`, confirm with `Enter`. Your choice is saved for next time.

| Mode | You see | You type |
|------|---------|----------|
| Chinese → Pinyin | 汉字 | pinyin (e.g. `hàn zì` or `han4zi4`) |
| Chinese → English | 汉字 | English meaning |
| English → Chinese | English | 汉字 |
| Pinyin → Chinese | pīnyīn | 汉字 |

**Step 2 — Content Select:** toggle HSK levels and custom decks with `Space`, confirm with `Enter`. Selection is saved for next time.

| Key | Action |
|-----|--------|
| `Space` | Toggle selected item |
| `a` | Select all |
| `n` | Deselect all |
| `Enter` | Start session |

Deck words bypass the SRS due-date filter — all cards are always available for practice. HSK words are still subject to normal scheduling even if they belong to a deck.

---

#### Prompt phase

| Key | Action |
|-----|--------|
| Type | Enter your answer |
| `Enter` | Submit and check |
| `Backspace` | Delete last character |
| `Esc` | Skip card |

**Pinyin input:** tone numbers are accepted — type `ni3hao3` or `ni3 hao3` and it converts to `nǐ hǎo`. Toned diacritics can also be typed or pasted directly.

| Tone | Mark | Type | Sound |
|------|------|------|-------|
| 1 | ā | a1 | flat / high |
| 2 | á | a2 | rising |
| 3 | ǎ | a3 | dipping |
| 4 | à | a4 | falling |

---

#### Grade phase

After submitting, the app suggests a grade based on your answer. Adjust with `←`/`→` and confirm with `Enter`.

| Key | Action |
|-----|--------|
| `←`/`h`  `→`/`l` | Move grade cursor |
| `0`–`5` | Jump to grade directly |
| `Enter` | Confirm highlighted grade |
| `y` | Copy hanzi to clipboard |
| `i` | Open word in MDBG browser dictionary |
| `s` | Suspend word (skip in all future sessions) |
| `Esc` | Return to main menu |

#### SM-2 grade guide

| Grade | Meaning | Approx. next review |
|-------|---------|---------------------|
| 0 Blackout | Completely forgot | ~1 hour |
| 1 Wrong | Incorrect | ~2.5 hours |
| 2 Hard | Wrong but recalled on seeing answer | ~12 hours |
| 3 Okay | Correct with difficulty | next day |
| 4 Good | Correct after a pause | interval × ease factor |
| 5 Perfect | Instant recall | longer interval |

The **Ease Factor (EF)** starts at 2.5 and adjusts based on grades. Higher EF = longer gaps between reviews. Floor is 1.3. A word is **mature** once its interval reaches 21 days.

---

### Stats Screen

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Scroll level list |
| `r` | Refresh |
| `Esc` / `q` | Return to menu |

---

### Custom Decks

1. Go to **Add Word to Deck** and select or create a deck
2. In the **Search & Browse** screen, type to find words by hanzi, pinyin, or English
3. Navigate results with `↑`/`↓`, press `Enter` to add the highlighted word to your deck
4. A word can be added to multiple decks — existing entries are reused, not duplicated
5. Press `/` to search the MDBG online dictionary for words not in the local database

**Actions in Search & Browse:**

| Key | Action |
|-----|--------|
| `Enter` | Add highlighted word to current deck |
| `E` | Edit the highlighted custom word (hanzi, pinyin, english) |
| `S` | Toggle suspend / unsuspend for highlighted word |
| `R` | Remove highlighted word from current deck (word is kept) |
| `D` | Delete highlighted custom word permanently |
| `/` | Open online dictionary search |

---

### Online Dictionary (MDBG)

From the Search screen, press `/` to open the online dictionary lookup. Searches run in the background — the UI stays responsive.

| Key | Action |
|-----|--------|
| `Enter` | Search online |
| `↑`/`↓` | Navigate results |
| `/` | Filter current results |
| `Enter` on result | Add word to deck |
| `Esc` | Back |

---

## Uninstalling

```bash
# Linux
rm -rf ~/.local/share/zhli ~/.config/zhli

# macOS
rm -rf ~/Library/Application\ Support/zhli ~/.config/zhli
```

---

## License

GNU General Public License v3.0 — see [LICENSE](LICENSE) for the full text.
