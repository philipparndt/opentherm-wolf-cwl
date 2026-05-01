## Context

Current repo structure:
```
opentherm-wolf-cwl/
├── esp32/          ← C++ PlatformIO project (legacy, unused)
│   ├── src/        ← C++ source
│   ├── web/        ← Preact SPA (actively used by Rust build)
│   ├── data/       ← web build output (SPIFFS content)
│   └── ...
├── esp32-rust/     ← Rust firmware (active project)
│   ├── src/
│   ├── Makefile    ← references ../esp32/web for web build
│   └── ...
├── openspec/       ← root openspec (references esp32-rust/)
└── ...
```

The Rust Makefile currently uses `WEB_SRC := ../esp32/web` and `WEB_OUT := ../esp32/data` to build the web UI and create the SPIFFS image.

## Goals / Non-Goals

**Goals:**
- Remove the legacy C++ project entirely
- Move web source into the Rust project directory
- Rename `esp32-rust` → `esp32` for a clean structure
- Preserve git history via `git mv` where possible
- Update all internal path references

**Non-Goals:**
- Changing any Rust source code or web UI code
- Modifying build logic beyond path adjustments
- Restructuring the web project internally

## Decisions

### 1. Operation order: move web first, delete C++, then rename

**Choice**: 
1. `git mv esp32/web esp32-rust/web` — move web into Rust project
2. `git rm -r esp32/` — remove the C++ project  
3. `git mv esp32-rust esp32` — rename to final location

**Rationale**: Moving web first preserves its git history cleanly. Deleting C++ after extraction ensures nothing is lost. The final rename gives us the clean `esp32/` directory.

### 2. Web build output stays at `./data` relative to the project

**Choice**: Update Makefile to `WEB_SRC := web` and `WEB_OUT := data`. The SPIFFS image is built from `data/` which will be at `esp32/data/`.

**Rationale**: Keeps the web build self-contained within the project. No more cross-directory references.

### 3. Root openspec directory handling

**Choice**: The root `openspec/` directory stays at the repo root (it's repo-level configuration). The `esp32-rust/openspec/` directory moves with the rename to `esp32/openspec/`.

### 4. Handle .gitignore updates

**Choice**: Check root `.gitignore` for any `esp32-rust/` or `esp32/data` paths and update them to reflect the new `esp32/` structure.

## Risks / Trade-offs

- **[Breaking external references]** → Anyone with scripts or CI referencing `esp32-rust/` will need updates. Mitigated by this being a personal project.
- **[Git blame on renamed files]** → `git mv` preserves history, `git log --follow` will track the renames.
