//! Embeds the icon + version info into the Windows executables, so Explorer, the
//! Start Menu and the taskbar show the Bunkr logo instead of the generic exe icon.

fn main() {
    println!("cargo:rerun-if-changed=winres/app.rc");
    println!("cargo:rerun-if-changed=assets/app-win.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("winres/app.rc", embed_resource::NONE);
    }
}
