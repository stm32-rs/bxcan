fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::copy("memory.x", format!("{}/memory.x", out_dir)).unwrap();
    println!("cargo:rustc-link-search={}", out_dir);
}
