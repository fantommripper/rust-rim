# rust-rim

A RimWorld mod manager written in Rust, built with [egui](https://github.com/emilk/egui).

![screenshot placeholder](assets/icon.png)

## Features

- **Dual-pane mod list** — inactive mods on the left, active load order on the right
- **Drag & drop reordering** — grab any mod and drop it into position
- **Search & filter** — live filtering in both panes simultaneously
- **Mod details panel** — shows description, version, author, supported versions and a preview image
- **Dependency & conflict warnings** — highlights missing dependencies (⚠) and active incompatibilities (✕)
- **Smart sort** — topological sort using `loadBefore`/`loadAfter` metadata with priority tiers (pre-Core patches → Core → DLC → frameworks → regular mods), optionally augmented by the [RimSort community rules database](https://github.com/RimSort/Community-Rules-Database)
- **Steam Workshop browser** — browse and search Workshop items by trending / latest / most subscribed / recently updated; browse and install collections
- **SteamCMD integration** — download Workshop mods directly without Steam; NixOS-aware (uses system `steamcmd` via `steam-run`)
- **Duplicate detection** — finds mods installed in multiple locations and lets you remove the extras
- **Persistent settings** — game path, config path, local mods path and SteamCMD location saved per user

## Building

Requires a recent stable Rust toolchain (edition 2024).

```bash
cargo build --release
```

The binary will be at `target/release/rust-rim`.

### NixOS

A `nix-shell` environment is recommended. The app automatically detects NixOS (via `/etc/NIXOS`) and uses the system `steamcmd` wrapped in `steam-run` instead of downloading its own copy.

### Dependencies (Linux)

egui uses `winit` + `wgpu` by default. You need standard graphics and windowing libraries:

- X11: `libx11`, `libxcb`, `libxi`, `libxkbcommon`
- Wayland: `wayland`, `libxkbcommon`
- GPU: Vulkan or OpenGL drivers

On NixOS these are provided by the shell environment.

## Usage

### First run

1. Launch `rust-rim`.
2. Open **Settings** (toolbar gear icon) and set:
   - **Game path** — the RimWorld installation directory (e.g. `~/.steam/steam/steamapps/common/RimWorld`)
   - **Config path** — the RimWorld config directory containing `ModsConfig.xml` (e.g. `~/.config/unity3d/Ludeon Studios/RimWorld by Ludeon Studios/Config`)
   - **Local mods path** — optional, your custom mods folder
3. Click **Scan** (or restart) to load mods. The active list is read from `ModsConfig.xml`.

### Managing mods

| Action | How |
|---|---|
| Activate / deactivate | Double-click a mod, or right-click → menu |
| Reorder | Drag & drop within the active list |
| Move one step | Right-click → Move up / Move down |
| Open mod folder | Right-click → Open folder |
| Filter | Type in the search box above either list |

### Sorting

Click **Sort** in the toolbar to auto-sort the active list. The sorter:

1. Puts pre-Core patches (Harmony, Prepatcher, etc.) first
2. Follows with Core and DLC in release order
3. Places framework mods (HugsLib, VFE Core, etc.) next
4. Resolves `loadBefore`/`loadAfter` constraints from mod metadata via Kahn's algorithm
5. Optionally applies community rules fetched from the RimSort database (toggle in Settings → Behavior)

### Steam Workshop

Click the **Workshop** button to open the browser. You can:
- Search or browse by sort order
- Switch between **Mods** and **Collections** tabs
- Click a collection to expand and queue all its items for download
- Download queued items via **SteamCMD** (installs automatically on first use, or uses system `steamcmd` on NixOS)

Downloaded mods land in `{steamcmd_path}/steam/steamapps/workshop/content/294100/` and are automatically picked up on next scan.

## Configuration

Settings are stored in the OS config directory:

| Platform | Path |
|---|---|
| Linux | `~/.config/rustrim/RustRim/settings.json` |
| macOS | `~/Library/Application Support/com.rustrim.RustRim/settings.json` |
| Windows | `%APPDATA%\rustrim\RustRim\config\settings.json` |

SteamCMD data defaults to the OS data directory (`~/.local/share/rustrim/RustRim/steamcmd_data` on Linux) unless overridden in Settings.

Logging is controlled via the `RUST_LOG` environment variable (default: `warn`):

```bash
RUST_LOG=info cargo run
```

## License

Modified MIT
