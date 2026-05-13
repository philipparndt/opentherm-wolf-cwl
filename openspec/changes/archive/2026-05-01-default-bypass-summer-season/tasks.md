## 1. Change BypassSchedule default

- [x] 1.1 Replace the derived `Default` on `BypassSchedule` with a manual `impl Default` that sets `enabled: true, start_day: 1, start_month: 4, end_day: 15, end_month: 9`

## 2. Verify

- [x] 2.1 Run `cargo check` to confirm compilation
