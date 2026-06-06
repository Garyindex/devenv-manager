# Contributing

Thanks for your interest in contributing to DevEnv Manager.

## Development

Install dependencies:

```bash
npm ci
```

Run the app in development:

```bash
npm run tauri:dev
```

Build the frontend:

```bash
npm run build
```

Build desktop bundles:

```bash
npm run tauri:build
```

## Checks

Before opening a pull request, run:

```bash
npm run build
npx tsc --noEmit
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Cleanup Rule Changes

Cleanup rules must be conservative.

- Prefer quarantine over permanent deletion.
- Classify risk honestly.
- Do not mark configuration, SDKs, toolchains, or project data as cleanable unless the behavior is well understood and recoverable.
- Add or update tests for rule parsing and safety behavior when possible.
