use std::env;

fn main() {
    embuild::espidf::sysenv::output();

    // Forward WiFi credentials from the build environment so the firmware can
    // read them via env!() at compile time. Empty strings are fine — the
    // runtime falls back to NVS-stored credentials when these are blank.
    println!("cargo:rerun-if-env-changed=WIFI_SSID");
    println!("cargo:rerun-if-env-changed=WIFI_PASSWORD");
    let wifi_ssid = env::var("WIFI_SSID").unwrap_or_default();
    let wifi_password = env::var("WIFI_PASSWORD").unwrap_or_default();
    println!("cargo:rustc-env=WIFI_SSID={}", wifi_ssid);
    println!("cargo:rustc-env=WIFI_PASSWORD={}", wifi_password);
}
