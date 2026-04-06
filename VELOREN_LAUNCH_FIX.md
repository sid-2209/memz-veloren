# 🔧 Veloren Launch Fix - Assets Directory Issue

## ❌ Problem

When running `./target/release/veloren-voxygen` directly, you get:
```
Asset directory not found. In attempting to find it, we searched:
/Users/siddhartha/Downloads/dev/Project Vyuh/memz/veloren/target/release/
```

## ✅ Solution

Veloren needs to be run from the repository root directory (where the `assets/` folder is located), not from `target/release/`.

### Option 1: Use the Helper Script (Recommended)

```bash
cd veloren
./run_veloren.sh
```

The script automatically runs the binary from the correct directory.

### Option 2: Run Directly from Repo Root

```bash
cd veloren
./target/release/veloren-voxygen
```

Make sure you're in the `veloren/` directory (where `assets/` folder exists), not in `veloren/target/release/`.

## 🎮 Complete Testing Instructions

### Terminal 1 - Start Ollama:
```bash
ollama serve
```

### Terminal 2 - Launch Veloren:
```bash
cd veloren
./run_veloren.sh
```

### In-Game:
1. Create/load character
2. Find any NPC
3. Press V, speak, release V
4. See response in chat!
5. Hear NPC voice!

## 📁 Directory Structure

```
veloren/
├── assets/          ← Veloren needs to find this!
├── target/
│   └── release/
│       └── veloren-voxygen  ← Binary location
├── run_veloren.sh   ← Helper script (run from here)
└── ...
```

## ✅ Verification

You'll know it's working when:
- Veloren launches without "Asset directory not found" error
- You see the main menu
- You can create/load a character
- You can press V to test voice

---

**Ready to test!** Just run `./run_veloren.sh` from the `veloren/` directory.
