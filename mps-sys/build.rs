use std::env;
use std::path::Path;

fn should_debug() -> bool {
    match env::var_os("DEBUG") {
        Some(s) => s != "false",
        None => false
    }
}

fn main() {
    let mut cc = cc::Build::new();
    cc.file("mps/code/mps.c");
    if should_debug() {
        cc.define("CONFIG_VAR_COOL", None);
    }
    cc.compile("mps");
    let bindings = bindgen::Builder::default()
        .header("mps/code/mps.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .whitelist_type("mps_.*")
        .whitelist_function("mps_.*")
        .whitelist_var("mps_.*")
        .generate()
        .expect("Unable to generate automatic bindings");
    bindings.write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap())
        .join("mps_auto.rs"))
        .expect("Failed to write bindings");
}