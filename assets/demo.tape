# atlas demo — record with charmbracelet/vhs:
#   brew install vhs           # pulls in ttyd + ffmpeg
#   cargo build --release      # build the atlas binary
#   vhs assets/demo.tape       # run from the repo root
# Produces assets/atlas-demo.gif — drop it at the top of the README, above Install.

Output assets/atlas-demo.gif

Set Theme "Catppuccin Mocha"
Set FontSize 18
Set Width 1100
Set Height 680
Set Padding 24
Set TypingSpeed 35ms
Set PlaybackSpeed 1.0

# Make the freshly built binary resolvable, without recording the setup.
Hide
Type "export PATH=$PWD/target/release:$PATH"
Enter
Type "clear"
Enter
Show

Type "# one command maps your whole repo — signatures only, no bodies"
Sleep 500ms
Enter
Type "atlas . --color always | head -16"
Sleep 400ms
Enter
Sleep 3s
Ctrl+L

Type "# --focus boosts what you're working on to the top"
Sleep 500ms
Enter
Type "atlas . --focus src/rank.rs --color always | head -16"
Sleep 400ms
Enter
Sleep 3s
Ctrl+L

Type "# pipe the map straight into your coding agent's context"
Sleep 500ms
Enter
Type "atlas . > atlas-map.md"
Sleep 400ms
Enter
Type "wc -l atlas-map.md"
Sleep 400ms
Enter
Sleep 2.5s
