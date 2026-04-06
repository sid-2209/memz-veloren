# 🎮 Veloren Setup Guide for Mac Mini M4

**System:** Mac Mini M4 (Apple Silicon)  
**Goal:** Run vanilla Veloren game (without MEMZ memory layer)  
**Date:** March 22, 2026

---

## Prerequisites Check

✅ **Rust:** You have Rust 1.93.0 installed  
✅ **Cargo:** Available at `/Users/siddhartha/.cargo/bin/cargo`  
✅ **Architecture:** ARM64 (Apple Silicon M4)

---

## Quick Start (Recommended)

### Option 1: Use Airshipper (Official Launcher) - EASIEST

The easiest way to play Veloren is using Airshipper, the official launcher that automatically downloads and updates the game.

1. **Download Airshipper for macOS:**
   ```bash
   # Visit the download page
   open https://veloren.net/download
   ```

2. **Or download directly:**
   ```bash
   # Download the macOS ARM64 version
   curl -L -o Airshipper.dmg "https://github.com/veloren/Airshipper/releases/latest/download/airshipper-macos-aarch64.dmg"
   
   # Open the DMG
   open Airshipper.dmg
   ```

3. **Install and Run:**
   - Drag Airshipper to Applications
   - Open Airshipper
   - Click "Download" to get the latest Veloren build
   - Click "Play" to launch the game

**Advantages:**
- Automatic updates
- No compilation needed
- Always compatible with official servers
- Takes 5 minutes to set up

---

## Option 2: Compile from Source (Your Local Copy)

If you want to compile the game yourself from the `veloren/` folder you have:

### Step 1: Navigate to Veloren Directory

```bash
cd veloren
```

### Step 2: Check System Dependencies

Veloren requires some system libraries. Install them via Homebrew:

```bash
# Install Homebrew if you don't have it
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install required dependencies
brew install cmake pkg-config python3
```

### Step 3: Build the Game Client (Voxygen)

The game client is called "voxygen". Build it with:

```bash
# Build in release mode (optimized, takes 10-20 minutes first time)
cargo build --release --bin veloren-voxygen

# Or use the cargo alias for faster development builds (2-5 minutes)
cargo run --bin veloren-voxygen
```

**Build Profiles:**
- `cargo build --release` - Optimized for performance (recommended for playing)
- `cargo run --bin veloren-voxygen` - Faster compilation, good enough to play
- `cargo build --profile no_overflow` - Fast compile + good performance

### Step 4: Run the Game

After building, run:

```bash
# If you built with --release
./target/release/veloren-voxygen

# Or if you used cargo run, it will start automatically
cargo run --bin veloren-voxygen
```

### Step 5: First Launch Setup

1. **Create an Account (Optional but Recommended):**
   - Visit https://veloren.net/account
   - Create a free account to play on official servers
   - This allows you to save your character across servers

2. **Game Launch:**
   - The game will open with a main menu
   - Click "Singleplayer" to play offline
   - Or click "Multiplayer" to join official servers

3. **Character Creation:**
   - Choose your race, body type, and appearance
   - Name your character
   - Start playing!

---

## Option 3: Download Pre-built Binary

If you don't want to compile, you can download pre-built binaries:

```bash
# Create a directory for Veloren
mkdir -p ~/Games/Veloren
cd ~/Games/Veloren

# Download the latest macOS ARM64 build
# Visit: https://veloren.net/download
# Or use Airshipper (recommended)
```

---

## Troubleshooting

### Issue 1: "Cannot find assets" Error

**Symptom:** Game crashes with "Failed to load assets"

**Solution:** Make sure you're running the game from the `veloren/` directory:

```bash
cd veloren
cargo run --bin veloren-voxygen
```

Or if using the compiled binary:

```bash
cd veloren
./target/release/veloren-voxygen
```

### Issue 2: Compilation Takes Too Long

**Symptom:** Build takes 30+ minutes

**Solution:** Use a faster build profile:

```bash
# Use the no_overflow profile (good balance)
cargo build --profile no_overflow --bin veloren-voxygen
./target/no_overflow/veloren-voxygen

# Or just use cargo run (dev profile)
cargo run --bin veloren-voxygen
```

### Issue 3: "Incompatible with server" Error

**Symptom:** Can't join multiplayer servers

**Solution:** Your local build might be outdated. Either:
- Use Airshipper to get the latest version
- Pull latest changes: `git pull origin master` and rebuild

### Issue 4: Low FPS / Performance Issues

**Symptom:** Game runs slowly

**Solutions:**
1. Make sure you built with `--release` or `--profile no_overflow`
2. Lower graphics settings in-game (press F3 for debug menu)
3. Close other applications
4. The M4 should handle Veloren well, but initial shader compilation may cause stuttering

### Issue 5: Linker Errors on macOS

**Symptom:** Build fails with linker errors

**Solution:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Make sure you have the latest Rust
rustup update
```

---

## Game Controls

### Basic Controls
- **WASD** - Move
- **Space** - Jump
- **Shift** - Sprint
- **Left Click** - Primary attack
- **Right Click** - Secondary attack
- **Q** - Roll/Dodge
- **E** - Interact
- **R** - Sheathe/Unsheathe weapon
- **B** - Bag/Inventory
- **C** - Character sheet
- **M** - Map
- **Enter** - Chat
- **F1** - Toggle UI
- **F2** - Take screenshot
- **F3** - Debug info

### Advanced Controls
- **Tab** - Social menu
- **N** - Crafting
- **L** - Lantern
- **G** - Glider
- **Z** - Sit/Stand
- **T** - Trade
- **P** - Group

---

## Performance Tips

### For Mac Mini M4

Your M4 should run Veloren very well! Here are some tips:

1. **Use Release Build:**
   ```bash
   cargo build --release --bin veloren-voxygen
   ```

2. **Graphics Settings:**
   - Start with "High" preset
   - If you get 60+ FPS, try "Ultra"
   - View distance: 10-15 chunks is good
   - Shadow quality: Medium or High

3. **Monitor Performance:**
   - Press F3 in-game to see FPS
   - Press F4 for detailed debug info

---

## Connecting to Servers

### Official Server
- Server address: `server.veloren.net`
- Requires account from https://veloren.net/account
- Always up-to-date
- Active community

### Singleplayer
- No internet required
- Full game experience
- Your own world
- Can enable cheats for testing

---

## File Locations

### Game Directory Structure
```
veloren/
├── assets/          # Game assets (voxel models, textures, sounds)
├── voxygen/         # Game client source code
├── server-cli/      # Server source code
├── target/          # Compiled binaries
│   ├── release/     # Optimized builds
│   │   └── veloren-voxygen  # The game executable
│   └── debug/       # Development builds
└── Cargo.toml       # Project configuration
```

### Save Files (when running locally)
```
~/Library/Application Support/veloren/
├── voxygen/
│   ├── settings.ron     # Game settings
│   └── profile/         # Character profiles
└── userdata/
    └── server/
        └── saves/       # World saves (singleplayer)
```

---

## Building the Server (Optional)

If you want to run your own server:

```bash
cd veloren

# Build the server
cargo build --release --bin veloren-server-cli

# Run the server
./target/release/veloren-server-cli

# Or use the cargo alias
cargo server
```

Server will start on port 14004 by default.

---

## Updating Your Local Build

To get the latest changes:

```bash
cd veloren

# Pull latest changes from GitLab
git pull origin master

# Rebuild
cargo build --release --bin veloren-voxygen
```

**Note:** Your local copy might be a specific version. Check the commit:

```bash
git log -1 --oneline
```

---

## Recommended: Use Airshipper

For the best experience, I strongly recommend using **Airshipper** instead of compiling:

**Why Airshipper?**
- ✅ Always up-to-date with official servers
- ✅ No compilation time
- ✅ Automatic updates
- ✅ One-click launch
- ✅ Manages multiple profiles
- ✅ Built-in news and changelog

**Download:** https://veloren.net/download

---

## Quick Command Reference

```bash
# Navigate to Veloren
cd veloren

# Build and run (development, fast compile)
cargo run --bin veloren-voxygen

# Build optimized (release, slow compile, best performance)
cargo build --release --bin veloren-voxygen
./target/release/veloren-voxygen

# Build balanced (no_overflow, medium compile, good performance)
cargo build --profile no_overflow --bin veloren-voxygen
./target/no_overflow/veloren-voxygen

# Run server
cargo server

# Clean build artifacts (if you need to free space)
cargo clean
```

---

## Estimated Build Times on Mac Mini M4

Based on M4 performance:

| Build Type | First Build | Incremental Build |
|------------|-------------|-------------------|
| `cargo run` (dev) | 3-5 minutes | 10-30 seconds |
| `--profile no_overflow` | 8-12 minutes | 1-2 minutes |
| `--release` | 15-25 minutes | 2-5 minutes |

**Tip:** Use `cargo run` for quick testing, `--release` for actual playing.

---

## Next Steps

1. **Start with Airshipper** (easiest, recommended)
2. **Or compile locally** if you want to modify the game
3. **Join the community:**
   - Discord: https://veloren.net/discord
   - Zulip: https://veloren.net/zulip
   - Reddit: r/Veloren
4. **Read the wiki:** https://wiki.veloren.net
5. **Check the book:** https://book.veloren.net

---

## Important Notes

### About MEMZ Integration

The `veloren/` folder in your project is a **vanilla Veloren clone**. The MEMZ memory system you've built is in the parent directory (`memz-core/`, `memz-llm/`, `memz-veloren/`).

**To run vanilla Veloren (without MEMZ):**
- Follow this guide ✅

**To run Veloren WITH MEMZ:**
- You'll need to integrate the MEMZ crates into Veloren
- This requires modifying Veloren's source code
- See the integration plan in `docs/veloren-rtsim-hooks.md`
- This is a development task, not ready for playing yet

### Current Status

- ✅ Veloren game: Fully playable
- 🚧 MEMZ integration: In development (Phase 0-1)
- 📋 MEMZ + Veloren: Not yet integrated

---

## Support

If you encounter issues:

1. **Check Veloren's official docs:** https://book.veloren.net
2. **Ask on Discord:** https://veloren.net/discord
3. **Check GitLab issues:** https://gitlab.com/veloren/veloren/-/issues
4. **For MEMZ-specific questions:** Check the MEMZ documentation in the parent directory

---

**Enjoy playing Veloren!** 🎮🗡️🏰
