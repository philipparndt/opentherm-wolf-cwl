## 1. Move web source into Rust project

- [x] 1.1 Run `git mv esp32/web esp32-rust/web` to move the web UI source into the Rust project
- [x] 1.2 Move `esp32/data` into `esp32-rust/data` if it contains build output that should be preserved (or create the dir)

## 2. Remove C++ project

- [x] 2.1 Run `git rm -r esp32/` to remove the legacy C++ PlatformIO project entirely

## 3. Rename esp32-rust to esp32

- [x] 3.1 Run `git mv esp32-rust esp32` to rename the Rust project to its final location

## 4. Update internal paths

- [x] 4.1 Update `Makefile` — change `WEB_SRC := ../esp32/web` to `WEB_SRC := web` and `WEB_OUT := ../esp32/data` to `WEB_OUT := data`
- [x] 4.2 Check and update root `.gitignore` for any `esp32-rust/` or `esp32/` path references
- [x] 4.3 Check and update root `README.md` if it references the old directory structure (no changes needed)
- [x] 4.4 Update `vite.config.ts` in web/ if it has output path references (already correct — `../data` relative to `web/`)

## 5. Verify

- [x] 5.1 Confirm the directory structure is correct: `esp32/` contains Cargo.toml, src/, web/, Makefile
- [x] 5.2 Run `cargo check` from the new `esp32/` directory
- [x] 5.3 Run `npm run build` from `esp32/web/` and confirm output goes to `esp32/data/`
