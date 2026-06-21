# Windows install & usage

atlas ships native Windows binaries. This guide covers the Windows-specific bits
the main [README](../README.md) keeps short: `PATH`, PowerShell, and shell
completions. Commands below are written for **PowerShell** unless noted.

> Some steps depend on your exact Windows version, shell, and security software;
> sections that haven't been verified end-to-end on a clean machine are marked
> **(unverified)**.

## Option A — pip / pipx (no Rust)

If you have Python, this is the easiest path. The PyPI package is `atlas-map`,
but the command it installs is `atlas`:

```powershell
pipx install --pre atlas-map    # isolated, recommended
# or:
pip install --pre atlas-map
```

`--pre` is required while atlas is in alpha. Then:

```powershell
atlas --version
```

If `atlas` isn't found afterward, see [PATH didn't update](#path-didnt-update).

## Option B — prebuilt release zip (no Python, no Rust)

1. Download `atlas-x86_64-pc-windows-msvc.zip` from the
   [releases page](https://github.com/fkenmar/atlas/releases) (use the `aarch64`
   zip on Arm devices).
2. Extract it, e.g. to `C:\Tools\atlas`.
3. Add that folder to your `PATH` so `atlas` works from any directory:

   ```powershell
   # Per-user, persists across sessions:
   [Environment]::SetEnvironmentVariable(
     "Path",
     [Environment]::GetEnvironmentVariable("Path", "User") + ";C:\Tools\atlas",
     "User"
   )
   ```

   Open a **new** terminal afterward (PATH changes don't apply to already-open
   shells), then verify:

   ```powershell
   atlas --version
   ```

## Option C — from source (needs Rust)

Install [Rust](https://rustup.rs), then:

```powershell
git clone https://github.com/fkenmar/atlas
cd atlas
cargo install --path .
```

This builds `atlas.exe` into `%USERPROFILE%\.cargo\bin`, which rustup adds to
`PATH` by default.

## Usage in PowerShell

The commands are the same as everywhere else; only redirection/piping syntax is
PowerShell-flavored:

```powershell
atlas .                              # map the current folder
atlas . --budget 4096                # bigger budget
atlas . --focus src\auth             # Windows path separators are fine
atlas . -o atlas-map.md              # write to a file (recommended over piping)
atlas . | Out-File -Encoding utf8 map.md   # explicit UTF-8 redirect
```

**Prefer `-o`/`--output` over `>`.** PowerShell's `>` historically writes UTF-16;
`atlas -o map.md` writes the file itself (UTF-8), avoiding encoding surprises
when an agent reads it. To pipe into another tool, `Out-File -Encoding utf8` or
`atlas .` straight into the command keeps it UTF-8.

## Shell completions

atlas generates a PowerShell completion script:

```powershell
atlas --completions powershell | Out-File -Encoding utf8 $PROFILE.CurrentUserAllHosts
# then reload:
. $PROFILE.CurrentUserAllHosts
```

Or append to your existing profile instead of overwriting it. **(unverified)**
For other shells on Windows (Git Bash, etc.), use the matching value:
`atlas --completions bash`.

## Troubleshooting

### `atlas` is not recognized

The install folder isn't on `PATH`. For pip/pipx, run `pipx ensurepath` (or check
where `pip` put scripts: `python -m site --user-base`, then `\Scripts`). For the
zip, redo the `PATH` step above. Always open a **new** terminal after changing
`PATH`.

### PATH didn't update

Environment-variable changes only apply to terminals opened **after** the change.
Close and reopen PowerShell (or sign out/in). Confirm the entry took:

```powershell
$env:Path -split ';' | Select-String atlas
```

### Running the completion script is blocked (execution policy)

If sourcing a profile/completion script errors with *"running scripts is
disabled on this system"*, relax the policy for your user:

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

`RemoteSigned` allows local scripts while still blocking unsigned downloaded
ones. **(unverified)** — adjust to your organization's policy.

### Antivirus / SmartScreen quarantine

A freshly downloaded, unsigned binary can be flagged by Windows SmartScreen or
antivirus. If `atlas.exe` is quarantined or "couldn't run," unblock it:

```powershell
Unblock-File C:\Tools\atlas\atlas.exe
```

Prefer the pip/pipx or `cargo install` paths if your environment restricts
unsigned executables. **(unverified)** — behavior depends on your security
software.

---

Back to the main [README](../README.md#install). For CI and exit-code behavior on
Windows runners, the [exit-code contract](exit-codes.md) is identical across
platforms.
