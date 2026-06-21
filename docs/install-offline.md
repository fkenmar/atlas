# Installing in restricted / corporate / offline networks

atlas runs **fully offline after installation** — it makes no network calls
during map generation and has no telemetry (see [`docs/PRIVACY.md`](PRIVACY.md)).
The only network step is *getting the binary onto the machine*. This guide covers
locked-down environments: proxies, air-gapped hosts, and internal mirrors.

## Pick the path that fits your restriction

| Situation                                   | Best option                              |
| ------------------------------------------- | ---------------------------------------- |
| Outbound HTTPS via a corporate **proxy**    | any normal install + [proxy vars](#proxies) |
| **Air-gapped** (no internet on the host)    | [download elsewhere, transfer](#air-gapped-transfer) |
| Internal **PyPI mirror** / Artifactory      | [pip from your index](#internal-pypi-mirror) |
| Rust toolchain available, no prebuilt allowed | [build from source](#build-from-source)  |

## Proxies

The standard installers all honor the usual proxy environment variables:

```sh
export HTTPS_PROXY=http://proxy.corp.example:8080
export HTTP_PROXY=http://proxy.corp.example:8080
export NO_PROXY=localhost,127.0.0.1,.corp.example
```

- **curl installer** and **GitHub downloads** use `HTTPS_PROXY` directly.
- **pip** honors these and also accepts `pip install --proxy http://… --pre atlas-map`.
- **cargo** honors `HTTPS_PROXY`, or a `[http] proxy = "…"` entry in
  `~/.cargo/config.toml`.

These variables are standard across the tools involved; set them and use any
install method from the [README](../README.md#install).

## Air-gapped transfer

Download on a connected machine, copy the artifact across, install locally.

**Prebuilt binary (no Python/Rust on the target):**

1. On a connected machine, grab the right asset from the
   [releases page](https://github.com/fkenmar/atlas/releases) — e.g.
   `atlas-x86_64-unknown-linux-gnu.tar.xz`,
   `atlas-aarch64-apple-darwin.tar.xz`, or
   `atlas-x86_64-pc-windows-msvc.zip`.
2. Transfer it to the target host.
3. Extract and put the `atlas` binary on your `PATH`
   (e.g. `~/.local/bin` or `/usr/local/bin`; on Windows see the
   [Windows guide](windows.md)).
4. Verify: `atlas --version`.

**pip wheel (target has Python, no internet):**

```sh
# On a connected machine:
pip download --pre atlas-map -d ./atlas-wheels
# Transfer ./atlas-wheels to the target, then:
pip install --no-index --find-links ./atlas-wheels --pre atlas-map
```

`--no-index --find-links` makes pip install only from the local directory, never
the network. The wheel drops the native `atlas` command onto `PATH`.

## Internal PyPI mirror

If your org proxies PyPI through Artifactory / Nexus / devpi, point pip at it:

```sh
pip install --index-url https://pypi.corp.example/simple --pre atlas-map
```

(or set it permanently in `pip.conf` / `PIP_INDEX_URL`). Ask your platform team
to mirror the `atlas-map` project if it isn't already cached.

## Build from source

With a Rust toolchain (online once to fetch crates, or pre-vendored):

```sh
git clone https://github.com/fkenmar/atlas
cd atlas
cargo fetch          # downloads all crate deps once
cargo install --path . --offline   # builds with no further network access
```

For a fully air-gapped build, run `cargo vendor` on a connected machine, commit
the `vendor/` directory plus the printed `.cargo/config.toml`, transfer the
source tree, and build with `--offline`.

## Verifying downloads

Per-release **checksums and signatures** are tracked in
[#63](https://github.com/fkenmar/atlas/issues/63); once they ship, this section
will document the exact `sha256sum`/signature-verification commands. Until then:

- prefer the **pip wheel** or **`cargo install`** paths, which verify integrity
  through PyPI / crates.io;
- if you download a release asset directly, fetch it over HTTPS from the official
  `github.com/fkenmar/atlas` releases and confirm the file size against the
  release page.

## After install

Nothing else needs the network. atlas reads your repo and writes a map locally;
the parse cache lives in `.atlas/` at the repo root. See
[`SECURITY.md`](../SECURITY.md) for the read-only/offline guarantees and
[`docs/PRIVACY.md`](PRIVACY.md) for the privacy model.
