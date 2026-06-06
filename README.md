# DevEnv Manager

DevEnv Manager is a lightweight desktop app for scanning local developer environments, estimating cache and toolchain usage, and helping developers review cleanup candidates before taking action.

It is built for developers who want to understand where local development space is going without blindly deleting caches, SDKs, package stores, or toolchain data.

## Features

- Scan common developer environment directories and cache locations.
- Estimate folder sizes with bounded, fast startup scanning.
- Deep scan selected entries when more detail is needed.
- Review entries by risk level: keep, caution, and cleanable.
- Move cleanable entries into quarantine before permanent deletion.
- Restore quarantined items.
- Inspect PATH health and installed development tools.
- Export local scan reports.
- Check app updates from GitHub Releases.

## Safety Model

DevEnv Manager is designed to avoid silent destructive behavior.

- Cleanup candidates are classified by rule-based risk levels.
- Only entries marked as cleanable are eligible for cleanup actions.
- Quarantine is preferred before permanent deletion.
- Cleanup, restore, export, and related actions are recorded in a local audit log.
- Scan results stay local by default.

Always review paths before taking cleanup actions. Developer caches are usually safe to rebuild, but local configuration, SDKs, toolchains, package stores, and project data may be expensive or impossible to recover.

## Download

Prebuilt installers are published through GitHub Releases when available.

Windows packages currently include:

- Portable executable: `DevEnv Manager_<version>_x64-portable.exe`
- NSIS installer: `DevEnv Manager_<version>_x64-setup.exe`
- MSI installers:
  - `DevEnv Manager_<version>_x64_zh-CN.msi`
  - `DevEnv Manager_<version>_x64_en-US.msi`

The portable executable does not require an installer, but the app still stores quarantine metadata and audit logs in the current user's local app data paths.

## Build From Source

Requirements:

- Node.js
- npm
- Rust stable
- Tauri system dependencies for your platform

Install frontend dependencies:

```bash
npm ci
```

Build the frontend:

```bash
npm run build
```

Run the Tauri development app:

```bash
npm run tauri:dev
```

Build desktop bundles:

```bash
npm run tauri:build
```

## Platform Status

| Platform | Status |
| --- | --- |
| Windows | Local bundle build verified |
| macOS | GitHub Actions workflow configured |
| Linux | GitHub Actions workflow configured with Tauri Linux dependencies |

The release workflow builds on `windows-latest`, `macos-latest`, and `ubuntu-22.04`.

## Updates

The settings screen checks the latest GitHub Release and shows the current version, latest version, release asset, publish time, release page, and installer download link.

This is a manual update flow: the app opens the release page or installer download in the browser. It does not currently perform in-app automatic installation.

## Tech Stack

- Tauri 2
- Rust
- Svelte 5
- Vite
- TypeScript
- npm

## Development Checks

Recommended checks before a release:

```bash
npm run build
npx tsc --noEmit
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Repository Notes

Generated build outputs and dependency folders should not be committed:

- `node_modules/`
- `dist/`
- `src-tauri/target/`

## Contributing

Issues and pull requests are welcome.

Before opening a pull request, please make sure:

- The app behavior remains honest and non-destructive.
- New cleanup rules have clear risk classification.
- User-visible actions have loading, error, and failure states where relevant.
- Build and Rust checks pass.

## License

MIT
