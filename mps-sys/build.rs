use std::env;

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
}