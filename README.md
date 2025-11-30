# niri-burst-consume

Automatically groups windows opened in rapid succession into the same column stack in [niri](https://github.com/YaLTeR/niri).

## What it does

When you open multiple windows within 500ms of each other, they're automatically consumed into a single column stack. Perfect for browser windows, related applications, or any workflow where you spawn several windows at once.


## How it works

The deamon listens for new window events from niri. When a new window appears, it checks if it was opened within the last 500ms of another new window. If so, it moves the new window into the same column stack as the previous one.

The daemon monitors niri's event stream and intelligently handles:
- **Title changes**: Terminal windows updating titles (kitty → zsh → ~) are deduplicated
- **Already-grouped windows**: Skips windows that are already in the same column
- **Recent window tracking**: Remembers the last 50 windows to avoid re-processing

Idle resource usage should be negiligible, rust build and tokio settings were optimized.

## Installation

Using Cargo:

```bash
cargo install --path .
```

Running via the flake:

```bash
nix run github:clotodex/niri-burst-consume
```

In NixOS HomeManager - use the flakes module:
The NixOS HomeManager module has a systemd service that manages the deamon.

<todo document>

## Configuration

Edit `THRESHOLD_MS` in `src/main.rs` to change the clustering window (default: 500ms).

## Logging

Control verbosity with `RUST_LOG`:

```bash
# Debug mode - see all events
RUST_LOG=debug niri-burst-consume

# Info mode - see clustering activity
RUST_LOG=info niri-burst-consume

# Default - errors only
niri-burst-consume
```

## Future Improvements & Contributions

I am happy to maintain this while I am using niri - feel free to drop and issues and PRs.
This will likely not end up having many features :)

- Configurable threshold via command-line arguments or config file


## License

MIT
