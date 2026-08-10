# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## Repository overview

Single-repo project for running a WoW 3.3.5a (WotLK) private server with playerbots and provisioning the game client.

- `install-wow-wotlk.sh` — bash installer for the server side (Docker, AzerothCore + Playerbots).
- `README.md` / `README.ru.md` — user guide front page: GUI installer overview + links.
- `README.manual.md` / `README.manual.ru.md` — manual install guides (download client, add to Steam, set realmlist).
- `README.server.md` — server-side networking guide.
- `scripts/installer.py` — **legacy** Python/tkinter GUI installer prototype. To be replaced by the Rust installer built on this branch. Do not extend it.
- `build_docker_image.sh`, `transfer.sh` — server-side helpers.

The client guide previously pointed to a Google Drive folder for the ruRU localization; that folder is corrupted and must be replaced by properly hosted patch artifacts (see Milestone M5).

## Current branch: `feat/gui-installer`

Created from `main`. All work in this plan happens on this branch.

- `feat/installer` — old branch holding the Python/tkinter prototype; superseded.
- Keep the plan below in sync as the implementation evolves.

## Goal

Build a **proper, statically-linked GUI installer for Windows and Linux** that:

1. Downloads the WoW 3.3.5a client (ChromieCraft build) over BitTorrent (magnet link).
2. Extracts it into a user-chosen directory, with progress, resume, cancel and cleanup.
3. Optionally downloads and applies **language/localization patches** (ruRU first, others later) and sets the in-game locale in `WTF/Config.wtf`.
4. Lets the user **choose the game server** — default local `127.0.0.1`, or a custom `host[:port]` — and writes `set realmlist ...` into every installed locale's `realmlist.wtf`.
5. Guides the user through Steam integration (add non-Steam game, Proton note on Linux), offers desktop shortcuts, and finishes with a "Launch" action.

## Tech stack (decision + rationale)

**Rust + egui/eframe.**

Why not the existing Python/tkinter prototype:
- PyInstaller bundles the interpreter but is not truly static; tkinter needs system Tcl/Tk; the `libtorrent` C++ bindings break static linking and complicate cross-compilation.
- Python apps force end users to have a functioning Python env, which WoW players cannot be expected to have.

Component choices (all pure-Rust so the binary stays static):

| Concern | Crate | Notes |
| --- | --- | --- |
| GUI | `eframe`/`egui` | winit backend dlopens X11/Wayland/GL at runtime, so the Linux build links glibc (see below) |
| Torrent | `librqbit` (rqbit) | magnet link, progress/peers, pause/resume, cancel; pure Rust |
| HTTP | `reqwest` with `rustls` | no OpenSSL dependency |
| ZIP | `zip` + `flate2` (miniz_oxide) | pure Rust backends, no zlib C dep |
| Config/persistence | `serde` + `toml` | installer settings, `locales.json` registry |
| Paths | `directories` | default install dirs per-OS |
| Checksums | `sha2` | verify localization patches / client integrity |

### Static linking definition

"Self-contained" = one executable with all app code, crates and Rust std linked in; **no Python, no external runtime, no app-level DLLs/.so files**. Only unavoidable OS facilities may be used at runtime:

- **Windows**: `-C target-feature=+crt-static` links the MSVC CRT statically where possible; a `VCRUNTIME140`/`MSVCP140` import pulled in by a dependency is accepted (no assertion in CI).
- **Linux**: standard glibc-linked release build. winit must `dlopen` X11/Wayland/GL at runtime, and a fully static binary (musl) has no dynamic loader — `dlopen` always fails — so static musl cannot open a GUI. glibc is present on every desktop distro, so the glibc build runs anywhere a desktop exists.

## Project layout

```
installer/
  Cargo.toml
  .cargo/config.toml         # crt-static flags for the static Windows target (see below)
  locales.json               # locale patch registry: id -> {name, url, sha256, set_locale}
  src/
    main.rs                  # entry: parse CLI flags, launch GUI or headless mode
    app.rs                   # egui app + wizard state machine (Screen enum)
    core/                    # PURE logic, no GUI deps — fully unit-testable
      server.rs              # Server { name, host, port } model, default 127.0.0.1
      realmlist.rs           # read/edit realmlist.wtf idempotently, per-locale
      locale.rs              # Config.wtf SET locale editing, apply patch into Data/
      extract.rs             # zip extraction (top-level-folder handling, progress)
      check.rs               # disk space, existing-install detection, integrity
    engine/
      torrent.rs             # rqbit wrapper: magnet → progress events, cancel/resume
      http.rs                # reqwest downloader with progress + checksum verify
      events.rs              # DownloadProgress / ApplyProgress event enum
    ui/
      mod.rs                 # Screen enum + router
      welcome.rs             # language select (auto-detect EN/RU), EULA/notes
      dir.rs                 # install directory + disk space
      server.rs              # local vs custom server (host[:port])
      download.rs            # client download progress, pause/resume
      extract.rs             # extraction progress
      localization.rs        # pick locales, download & apply patches
      steam.rs               # Steam detection, guided add-to-Steam, desktop shortcut
      finish.rs              # summary, "Launch" button
    steam.rs                 # Steam dir detection (win/linux), shortcut creation
    logging.rs               # install.log next to binary, error capture
  tests/                     # integration tests using a small fake client zip
  assets/                    # icons, branding
```

## Key design decisions

- **Core UI separation**: everything in `core/` and `engine/` is GUI-free. The GUI (or a headless `--cli` mode) just drives it. This is what makes the logic unit-testable and lets CI run headless smoke tests.
- **Headless/CLI mode**: `installer --dir <path> --server <host[:port]> --locale ruRU --yes` for unattended installs and automated testing.
- **Locale-aware realmlist**: write `set realmlist <addr>` into *every* `Data/<locale>/realmlist.wtf` found in the client (enUS + ruRU + any applied patch). Editing is idempotent: replace existing `set realmlist` line, append if missing, keep other settings.
- **Default server**: `127.0.0.1` (matches the server installer `install-wow-wotlk.sh`). UI must state clearly that this is the local machine and that playing on a real server requires entering its IP.
- **Server port**: support `host:port` (non-default port needs explicit port in realmlist); omit port when empty/default.
- **Localization patches**: hosted on GitHub Releases (NOT Google Drive — currently corrupted). `locales.json` maps locale → `{ url, sha256, set_locale }`. After apply: verify checksum, merge `Data/<locale>/` into the client, write `SET locale "<locale>"` into `WTF/Config.wtf`.
- **Existing-install / repair mode**: if `WoW.exe` already exists in the target dir, offer "Repair / Reconfigure" which skips download+extract and only re-applies locale + realmlist config.
- **Cancellation safety**: every long-running step checks a cancel flag; temp dir under the install dir is removed on cancel/failure (mirrors the prototype's behavior).
- **Steam integration is guided, not invasive**: detect Steam install, show clear instructions (and optionally create a `.lnk`/`.desktop` shortcut + auto-open Steam Library). Do NOT hand-edit `shortcuts.vdf` (brittle, locked when Steam runs). Keep the Proton note for Linux from the README.

## Static linking: build flags

`.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static"]
```

The Linux musl target is only usable for headless `--cli` builds (no GUI). The shipped
Linux binary is a standard glibc release build (host target, no `--target` flag).

Runtime deps must stay C-free: verify `flate2` uses `miniz_oxide`, `reqwest` uses `rustls-tls`, no `openssl-sys`/`libtorrent2`.

## Milestones

- **M0 — Scaffold**: create `installer/` crate, `.cargo/config.toml`, GitHub Actions skeleton (matrix build linux-gnu + windows-msvc), `--version` smoke test, README section.
- **M1 — Core config logic**: `core::server`, `core::realmlist`, `core::locale` with full unit tests (this is the "choose server / apply language" heart — build it first, GUI-agnostic).
- **M2 — Download engine**: `engine::torrent` (rqbit, magnet, progress events, cancel/resume) + `engine::http` (progress, checksum). Headless test with a tiny seeded torrent fixture.
- **M3 — Extraction**: `core::extract` with progress, top-level-folder detection/stripping, `zip` crate. Unit-tested with a synthetic client zip.
- **M4 — GUI wizard**: egui app with all screens wired to core/engine. Focus on the install flow end-to-end (download → extract → finish) before localization.
- **M5 — Localization pipeline**: ship `locales.json` + patch artifacts (ruRU first) on GitHub Releases (or a Google Drive share link, which the HTTP engine resolves and SHA-256-verifies), apply + `Config.wtf` locale logic wired into the wizard.
- **M6 — Server selection + Steam**: server choice screen (default 127.0.0.1, custom host[:port], persists last used), realmlist writes into all locales, Steam detection + guided integration + desktop shortcuts, finish screen with Launch.
- **M7 — Release hardening**: static-verification CI step, matrix release on tags, docs update (README/README.ru.md), remove `scripts/installer.py` + `scripts/README.md`, cleanup `feat/installer` reference.

## Build / verify commands

```sh
# Unit + integration tests (host)
cargo test --manifest-path installer/Cargo.toml

# Linux release build (glibc — required for the GUI; winit dlopens X11/Wayland/GL)
cargo build --release --manifest-path installer/Cargo.toml

# Windows crt-static build (native on windows runner; or via cargo-xwin from Linux)
cargo build --release --manifest-path installer/Cargo.toml --target x86_64-pc-windows-msvc

# Headless smoke test of the binary
./installer/target/release/wow_installer --version
```

## Testing strategy

- **Unit tests** (M1, M3, M5): realmlist editing (replace/append/idempotent, per-locale), Config.wtf locale switching, zip extraction with/without top-level folder, checksum verify.
- **Engine tests** (M2): download a tiny local torrent/HTTP fixture headless, assert progress events, cancel mid-download cleans temp files.
- **GUI**: manual QA matrix — Windows 10/11, Steam Deck (SteamOS/Arch), Ubuntu, Fedora. `--cli` path tested in CI.
- **Static check**: CI builds the Windows `crt-static` target and verifies the binary runs; shared MSVC CRT imports are tolerated.

## CI/CD (GitHub Actions)

- Reuse pattern from `feat/installer`'s `.github/workflows/release.yml` (matrix, tag-triggered releases) but with Rust toolchain and static targets.
- On `push` to `main` / PR: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, headless `--version` smoke.
- On tag `v*`: build matrix `ubuntu-latest` (glibc) + `windows-latest` (crt-static), upload `dist/` artifacts to the release, generate release notes.

## Acceptance criteria

1. One binary per OS (Windows `.exe`, Linux ELF) with no runtime installs — Windows `crt-static`, Linux glibc-linked.
2. Full flow works end-to-end: download → extract → optional localization → choose server → realmlist written to all locales → Steam guidance → Launch.
3. Default server is `127.0.0.1`, and the UI clearly explains how/when to change it.
4. ruRU localization downloadable and correctly applied (Config.wtf `SET locale "ruRU"`). Google Drive share links are supported by the HTTP engine (share-link → confirm-token → download) **but always SHA-256-verified** — the old unverified Drive folder was corrupted, which is why verification is mandatory.
5. Cancel/resume/repair behave correctly and temp artifacts are cleaned up.
6. Windows build uses `crt-static` (shared MSVC CRT imports are tolerated); the Linux build is a glibc release so `dlopen` of X11/Wayland works on desktop distros.
7. README.md and README.ru.md updated to point users at the new installer.

## Conventions for agents

- Keep `core/` and `engine/` free of GUI dependencies; test them headlessly.
- Follow existing repo style: short, direct commits; docs are bilingual (README.md EN / README.ru.md RU).
- Do not commit unrelated working-tree changes (e.g. an uncommitted README note on `main`) onto this branch.
- UI copy must be bilingual (EN + RU) with auto-detect, matching the README structure.
- Never hand-edit Steam `shortcuts.vdf`; keep Steam integration guided.
