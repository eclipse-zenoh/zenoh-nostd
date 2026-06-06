fn main() {
    // Ensure memory.x is in the linker search path
    println!("cargo:rustc-link-search=.//");
    println!("cargo:rerun-if-changed=memory.x");
}