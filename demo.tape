# demo.tape — generates the README demo GIF.
#
#   vhs demo.tape        # writes demo.gif in the repo root
#
# Requires:
#   - vhs            https://github.com/charmbracelet/vhs  (brew install vhs)
#   - patent on PATH (cargo install --path .)
#   - Ollama running for the full AI verdict (`ollama serve`); without it the
#     TUI still renders with a similarity-only verdict, or add `--fast` below.
#
# The GIF is served to the README via its raw.githubusercontent.com URL, so
# commit demo.gif after generating it. It's excluded from the crate tarball.

Output demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 720
Set Padding 24
Set Theme "Dracula"

# Let the prompt settle before typing.
Sleep 1s

Type `patent "interactive cli to kill whatever's on a port"`
Sleep 600ms
Enter

# Search → ranking → verdict happen off-screen (Hide) so the README loop isn't
# half spinner. The command + Enter stay visible above; the TUI just appears.
# Bump the sleep if your machine or model is slower (warm ~5.5s, cold ~9s).
Hide
Sleep 10s
Show

# Scroll through the ranked matches.
Down@350ms 5
Sleep 700ms

# Enter opens the detail popup — full description, popularity, and URL.
Enter
Sleep 2800ms
Escape
Sleep 500ms

# `s` re-ranks: similarity → popularity → name (watch the table title).
Type "s"
Sleep 1600ms
Type "s"
Sleep 1600ms
Type "s"
Sleep 1000ms

# Peek at the keybindings overlay, then close it.
Type "?"
Sleep 2200ms
Escape
Sleep 500ms

# Quit cleanly.
Type "q"
Sleep 1s
