fn main() {
    println!("cargo::rustc-env=CARGO_FEATURES=");
    println!("cargo::rustc-env=CARGO_CFG_TARGET_OS=");
    println!("cargo::rustc-env=CARGO_CFG_TARGET_ENV=");
    println!("cargo::rustc-env=CARGO_CFG_TARGET_ARCH=");
    println!("cargo::rustc-env=GIT_DESCRIBE=");
    println!("cargo::rustc-env=GIT_REV=");
}
