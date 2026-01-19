//! Build script for rx-lx-sys
//! Compiles the RandomX C++ library and links it statically.

fn main() {
    // Path al submodule RandomX
    let dst = cmake::Config::new("../../vendor/randomx")
        // Per build portabile (consenso), non usiamo ARCH=native
        // Per binari miner-only ottimizzati, si può usare .define("ARCH", "native")
        .build();

    // Directory dove CMake mette le librerie
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").display()
    );

    // Link della libreria RandomX (static)
    println!("cargo:rustc-link-lib=static=randomx");

    // Link C++ standard library (necessario su Linux per C++)
    println!("cargo:rustc-link-lib=dylib=stdc++");

    // Rebuild se cambia il codice RandomX
    println!("cargo:rerun-if-changed=../../vendor/randomx");
}
