use std::env;
use std::path::PathBuf;

fn main() {
    embuild::espidf::sysenv::output();

    // Compile OpenTherm FFI when not simulating
    if env::var("CARGO_FEATURE_SIMULATE_OT").is_err() {
        compile_opentherm();
    }
}

fn compile_opentherm() {
    // Find the esp-idf-sys build output directory which contains sdkconfig.h
    // and all the compiled ESP-IDF components
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR is like .../target/.../build/wolf-cwl-HASH/out
    // esp-idf-sys output is at .../target/.../build/esp-idf-sys-HASH/out
    let build_dir = out_dir.parent().unwrap().parent().unwrap(); // .../build/

    let mut esp_idf_out = None;
    if let Ok(entries) = std::fs::read_dir(build_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("esp-idf-sys-") {
                let candidate = entry.path().join("out/build/config/sdkconfig.h");
                if candidate.exists() {
                    esp_idf_out = Some(entry.path().join("out"));
                    break;
                }
            }
        }
    }

    let esp_idf_out = match esp_idf_out {
        Some(p) => p,
        None => {
            println!("cargo:warning=Could not find esp-idf-sys build output, skipping OpenTherm compilation");
            return;
        }
    };

    let build_config = esp_idf_out.join("build/config");
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let esp_idf_root = PathBuf::from(&manifest).join(".embuild/espressif/esp-idf");

    // Find ESP-IDF version directory
    let mut idf_components = None;
    if let Ok(entries) = std::fs::read_dir(&esp_idf_root) {
        for entry in entries.flatten() {
            let comp = entry.path().join("components");
            if comp.exists() {
                idf_components = Some(comp);
                break;
            }
        }
    }

    let idf_components = match idf_components {
        Some(p) => p,
        None => {
            println!("cargo:warning=Could not find ESP-IDF components, skipping OpenTherm compilation");
            return;
        }
    };

    // Find the Xtensa cross-compiler
    let toolchain_dir = PathBuf::from(&manifest)
        .join(".embuild/espressif/tools/xtensa-esp-elf");
    let compiler = find_file(&toolchain_dir, "xtensa-esp32-elf-g++")
        .expect("Could not find xtensa-esp32-elf-g++");

    let mut build = cc::Build::new();
    build
        .compiler(&compiler)
        .cpp(true)
        .file("components/opentherm/OpenTherm.cpp")
        .file("components/opentherm/opentherm_ffi.cpp")
        .include("components/opentherm") // Our Arduino.h shim
        .include(&build_config)          // sdkconfig.h
        .flag("-std=gnu++17")
        .flag("-fno-exceptions")
        .flag("-fno-rtti")
        .flag("-mlongcalls")
        .flag("-w");

    // Add ESP-IDF component include directories
    let includes = [
        "esp_common/include",
        "esp_hw_support/include",
        "esp_hw_support/include/soc",
        "esp_hw_support/include/soc/esp32",
        "esp_system/include",
        "esp_rom/include",
        "esp_rom/include/esp32",
        "esp_timer/include",
        "hal/include",
        "hal/esp32/include",
        "soc/include",
        "soc/esp32/include",
        "soc/esp32/register",
        "freertos/FreeRTOS-Kernel/include",
        "freertos/FreeRTOS-Kernel/portable/xtensa/include",
        "freertos/esp_additions/include",
        "freertos/esp_additions/include/freertos",
        "freertos/config/include",
        "freertos/config/xtensa/include",
        "driver/gpio/include",
        "esp_driver_gpio/include",
        "newlib/platform_include",
        "xtensa/include",
        "xtensa/esp32/include",
        "log/include",
        "heap/include",
    ];

    for inc in &includes {
        let path = idf_components.join(inc);
        if path.exists() {
            build.include(&path);
        }
    }

    // Also add the build output include (for generated headers)
    let build_inc = esp_idf_out.join("build/include");
    if build_inc.exists() {
        build.include(&build_inc);
    }

    build.compile("opentherm");
    println!("cargo:rerun-if-changed=components/opentherm/");
}

fn find_file(dir: &PathBuf, name: &str) -> Option<PathBuf> {
    if !dir.exists() { return None; }
    for entry in walkdir(dir) {
        if entry.file_name().map(|n| n.to_string_lossy().as_ref() == name).unwrap_or(false) {
            return Some(entry);
        }
    }
    None
}

fn walkdir(dir: &PathBuf) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}
