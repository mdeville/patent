# showcase.tape — tighter capture for a ratatui showcase submission.
#
#   vhs showcase.tape    # writes showcase.gif in the repo root
#
# Deliberately different from demo.tape. The showcase opens with the CLI
# invocation visible so the product name and usage are immediately clear,
# then hides the load wait so the TUI just appears — no spinner in the loop.
# See https://ratatui.rs/recipes/apps/submitting-to-the-showcase/
#
# Pre-warm Ollama before recording (`ollama serve` + one throwaway run) so the
# verdict is real LLM text, not the --fast similarity fallback.

Output showcase.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 720
Set Padding 24
Set Theme "Dracula"

# Show the CLI invocation — product name and usage are visible from frame one.
Type `patent "fuzzy finder for searching files and text in the terminal"`
Sleep 600ms
Enter

# Hide the load wait (search + ranking + verdict happen off-screen).
# Bump the sleep if your machine or Ollama model is slower.
Hide
Sleep 18s
Show

# Hero frame: ranked matches + verdict. Hold so it still reads when scaled down.
Sleep 3s

# Screenshot for showcase thumbnail (scales better than gif on small screens).
Screenshot showcase.png

# A little calm motion through the matches.
Down@350ms 4
Sleep 800ms

# Detail popup — shows description, popularity, URL.
Enter
Sleep 4500ms
Escape
Sleep 1200ms
