# 中 ZLI — Command Line Chinese Trainer

**Version 0.1.0**

A terminal-based flashcard app for learning HSK 1–6 Chinese vocabulary using the SM-2 spaced repetition algorithm. All data is stored locally in SQLite — no account, no internet required.

---

## Features

- HSK levels 1–6 (2 500+ vocabulary cards)
- Four study directions: Chinese→Pinyin, Chinese→English, English→Chinese, Pinyin→Chinese
- SM-2 spaced repetition scheduling
- Numbered pinyin input (`hao3` → `hǎo`)
- Custom word decks
- Stats dashboard with streak, review history, and weakest-word tracking
- Clipboard yank and browser dictionary lookup during review

---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later (install via `rustup` — recommended on all platforms)
- A terminal with 256-colour support (most modern terminals qualify)
- **Linux only:** clipboard support requires a clipboard utility at runtime (see below)

---

## Installation

### Arch Linux

```bash
# Install Rust via rustup (recommended) or pacman
sudo pacman -S rustup
rustup default stable

# Clipboard dependencies (pick one depending on your display server)
sudo pacman -S xclip          # X11
sudo pacman -S wl-clipboard   # Wayland

# Build-time XCB headers (needed to compile the clipboard crate)
sudo pacman -S libxcb

# Clone and build
git clone https://github.com/your-username/chinese-line-interface.git
cd chinese-line-interface
cargo build --release
```

### Ubuntu / Debian

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clipboard and build-time XCB headers
sudo apt install xclip libxcb1-dev libxcb-render0-dev \
                 libxcb-shape0-dev libxcb-xfixes0-dev
# Wayland alternative:
# sudo apt install wl-clipboard

# Clone and build
git clone https://github.com/your-username/chinese-line-interface.git
cd chinese-line-interface
cargo build --release
```

### macOS

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# No extra clipboard dependencies — macOS provides native clipboard access

# Clone and build
git clone https://github.com/your-username/chinese-line-interface.git
cd chinese-line-interface
cargo build --release
```

---

## Running

```bash
# Run the compiled binary
./target/release/zli

# Or run directly with cargo
cargo run --release
```

The app creates its database on first launch and seeds all HSK 1–6 words automatically.

**Data location:**

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/hsk-cli/data.db` |
| macOS    | `~/Library/Application Support/hsk-cli/data.db` |

---

## Usage

### Main Menu

Navigate with `↑`/`↓` or `j`/`k`, confirm with `Enter`, quit with `q`.

| Option | Description |
|--------|-------------|
| Study / Review Cards | Select level and mode, then start a session |
| Select Level & Mode | Configure without starting a session |
| Stats & Report Card | View progress, streaks, and weakest words |
| Add Word to Deck | Create or name a custom deck |
| Search & Browse | Search words and add them to a deck |
| Quit | Exit the app |

---

### Study Session

**Step 1 — Level Select:** choose HSK 1–6 with `↑`/`↓`, confirm with `Enter`.

**Step 2 — Mode Select:** pick one study direction with `↑`/`↓`, confirm with `Enter`.

| Mode | You see | You type |
|------|---------|----------|
| Chinese → Pinyin | 汉字 | pinyin (e.g. `hàn zì` or `han4zi4`) |
| Chinese → English | 汉字 | English meaning |
| English → Chinese | English | 汉字 |
| Pinyin → Chinese | pīnyīn | 汉字 |

#### Prompt phase

| Key | Action |
|-----|--------|
| Type | Enter your answer |
| `Enter` | Submit and check |
| `Backspace` | Delete last character |
| `Esc` | Skip card (counts as Wrong) |

**Pinyin input tip:** tone numbers are accepted — type `ni3hao3` or `ni3 hao3` and it converts to `nǐ hǎo`. Toned diacritics can also be pasted directly. A tone guide is shown on screen as a reminder.

| Tone | Mark | Type | Sound |
|------|------|------|-------|
| 1 | ā | a1 | flat / high |
| 2 | á | a2 | rising |
| 3 | ǎ | a3 | dipping |
| 4 | à | a4 | falling |

#### Grade phase

After submitting, the app suggests a grade based on your answer. You can accept it or adjust.

| Key | Action |
|-----|--------|
| `←`/`h`  `→`/`l` | Move grade cursor |
| `Enter` | Confirm highlighted grade |
| `0` | Blackout — no recall |
| `1` | Wrong |
| `2` | Hard |
| `3` | Okay |
| `4` | Good |
| `5` | Perfect |
| `y` | Copy the hanzi to clipboard |
| `i` | Open word in MDBG online dictionary |
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

The **Ease Factor (EF)** starts at 2.5 and adjusts up or down based on your grades. Higher EF = longer gaps between reviews. The floor is 1.3. A word is counted as **learned** once its interval reaches 21 days.

---

### Stats Screen

| Key | Action |
|-----|--------|
| `r` | Refresh |
| `Esc` / `q` | Return to menu |

Shows: daily streak, reviews today, overall retention, cards due, per-level progress bars (seen / learned), the 10 weakest words by ease factor, and a 7-day review activity chart.

---

### Custom Decks

1. Go to **Add Word to Deck**, type a deck name in the input field
2. Press `Tab` to open Search
3. In Search, type to find words by hanzi, pinyin, or English
4. Navigate results with `↑`/`↓`, press `Tab` to add the highlighted word to your deck

Words added to a custom deck can be studied by selecting that deck name at review time (selecting a named deck at the Level Select screen is a planned feature — currently all words at the chosen HSK level are included).

---

## Uninstalling

```bash
# Linux
rm -rf ~/.local/share/hsk-cli

# macOS
rm -rf ~/Library/Application\ Support/hsk-cli
```

---

## License

MIT
