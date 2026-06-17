# atlas demo — record with charmbracelet/vhs:
#   brew install vhs           # or: go install github.com/charmbracelet/vhs@latest
#   cd <your repo>             # run from a repo with real Python/TS/Rust source
#   vhs path/to/assets/demo.tape
# Produces assets/atlas-demo.gif — drop it at the top of the README, above Install.

Output assets/atlas-demo.gif

Set Theme "Catppuccin Mocha"
Set FontSize 18
Set Width 1100
Set Height 640
Set Padding 24
Set PlaybackSpeed 0.8

Type "atlas ."
Sleep 400ms
Enter
Sleep 3s

Type "# rank what you're working on higher, bigger budget"
Sleep 600ms
Enter
Type "atlas . --focus src --budget 4096 | head -20"
Sleep 400ms
Enter
Sleep 3s

Type "# save it and hand it to your agent"
Sleep 600ms
Enter
Type "atlas . > atlas-map.md"
Sleep 400ms
Enter
Sleep 1.5s

Sleep 2s
