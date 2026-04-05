# BreakTime

BreakTime is a lightweight macOS menu bar break reminder written in Rust. It lives in the system status bar, shows timer progress through the tray icon, and uses a two-stage reminder flow around the end of each countdown:

- A floating countdown bubble appears near the menu bar during the final 10 seconds
- When the timer expires, the app shows a full-screen mask and confirmation dialog to force a break prompt

This repository currently supports macOS only. On other platforms the binary exits immediately and prints an unsupported-platform message.

## Features

- Runs from the macOS menu bar without a Dock presence
- Updates the tray icon to reflect progress through the current interval
- Includes a built-in toggle to enable or pause reminders
- Includes a duration slider with a range from 1 to 20 minutes
- Shows a floating countdown bubble during the final 10 seconds
- Shows a modal break prompt with a full-screen overlay when the timer expires
- Supports `+1 min` to snooze for one minute
- Supports `Done` to immediately start the next interval

## Requirements

- macOS
- Rust stable toolchain
- A compiler with Rust 2024 edition support

If you do not already have Rust installed:

```bash
rustup toolchain install stable
rustup default stable
```

## Run Locally

```bash
cargo run
```

After launch, the app appears in the macOS menu bar. The default state is disabled, so you need to open the menu and turn on `Enabled`.

## Usage

1. Launch the app and click the BreakTime icon in the menu bar.
2. Turn on the `Enabled` switch to start the timer.
3. Use the `Duration` slider to set the reminder interval.
4. When the timer enters the last 10 seconds, a countdown bubble appears near the menu bar.
5. When the timer expires, the break dialog is shown.
6. Click `+1 min` to postpone the current prompt by one minute.
7. Click `Done` to dismiss the current prompt and start the next interval immediately.

## CI And Releases

This repository includes two GitHub Actions workflows:

- `CI`: runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` on every branch push and pull request
- `Release`: when you push a tag matching `v*`, GitHub Actions builds macOS release artifacts and publishes them to a GitHub Release

The release workflow uploads two runnable `.app.zip` bundles:

- Apple Silicon: `BreakTime-vX.Y.Z-macos-apple-silicon.app.zip`
- Intel: `BreakTime-vX.Y.Z-macos-intel.app.zip`

To publish a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

If you want the generated app bundle to use your own bundle identifier, set the repository variable `BREAKTIME_BUNDLE_ID` in GitHub before pushing the tag. The workflow falls back to `com.example.breaktime` if the variable is not set.

## Current Limitations

- macOS only
- No persistent configuration yet; the app returns to default state after restart
- The default reminder interval is 20 minutes, and the app starts disabled
- The bundle identifier in the repository is still a placeholder and should be replaced with your own reverse-domain identifier before distribution
