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
        .header("mps/code/mpsavm.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .whitelist_type("mps_.*")
        .whitelist_function("mps_.*")
        .whitelist_var("mps_.*")
        .whitelist_var("MPS_.*")
        .whitelist_var("_mps_key.*")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate automatic bindings");
    bindings.write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap())
        .join("mps_auto.rs"))
        .expect("Failed to write bindings");
}