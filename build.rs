//! Embeds the Windows resources into the DontTouchMeBro binary only: the app
//! manifest (requireAdministrator, DPI awareness), the three tray icons and
//! version information. Test binaries get none of it, so `cargo test` runs
//! without a UAC prompt.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const BINARY: &str = "DontTouchMeBro";

/// Icon resource IDs; must match the constants in src/tray.rs. The first
/// icon in the file is also the exe's icon in Explorer.
const ICONS: [(u16, &str); 3] = [
    (101, "assets/yes.ico"),
    (102, "assets/no.ico"),
    (103, "assets/error.ico"),
];

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-env-changed=DTMB_AS_INVOKER");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let parts = version_parts(&version);
    let dotted = parts.map(|p| p.to_string()).join(".");

    let mut manifest = fs::read_to_string(root.join("app.manifest"))
        .expect("read app.manifest")
        .replace("version=\"0.0.0.0\"", &format!("version=\"{dotted}\""));

    // Local testing only: run without the UAC prompt. Reading devices works
    // unelevated; enabling/disabling them will fail with "Access is denied".
    if env::var("DTMB_AS_INVOKER").as_deref() == Ok("1") {
        println!(
            "cargo:warning=DTMB_AS_INVOKER=1: building WITHOUT requireAdministrator (testing only)"
        );
        manifest = manifest.replace("level=\"requireAdministrator\"", "level=\"asInvoker\"");
    }
    let manifest_path = out_dir.join("app.manifest");
    fs::write(&manifest_path, manifest).expect("write manifest");

    let icons: String = ICONS
        .iter()
        .map(|(id, path)| format!("{id} ICON \"{}\"\n", rc_path(&root.join(path))))
        .collect();
    let commas = parts.map(|p| p.to_string()).join(",");
    let rc = format!(
        r#"#pragma code_page(65001)
{icons}
1 24 "{manifest}"

1 VERSIONINFO
FILEVERSION {commas}
PRODUCTVERSION {commas}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Down Right Technical Inc."
      VALUE "FileDescription", "Don't Touch Me Bro"
      VALUE "FileVersion", "{dotted}"
      VALUE "InternalName", "{BINARY}"
      VALUE "LegalCopyright", "Copyright (c) Down Right Technical Inc."
      VALUE "OriginalFilename", "{BINARY}.exe"
      VALUE "ProductName", "Don't Touch Me Bro"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        manifest = rc_path(&manifest_path),
    );
    let rc_file = out_dir.join("resources.rc");
    fs::write(&rc_file, rc).expect("write resources.rc");

    embed_resource::compile_for(&rc_file, [BINARY], embed_resource::NONE)
        .manifest_required()
        .expect("compile Windows resources");
}

/// `2.0.0` -> `[2, 0, 0, 0]` (Windows versions have four parts).
fn version_parts(version: &str) -> [u16; 4] {
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let mut parts = [0u16; 4];
    for (slot, part) in parts.iter_mut().zip(core.split('.')) {
        *slot = part.parse().expect("numeric version component");
    }
    parts
}

/// A path as an .rc string literal (backslashes doubled).
fn rc_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}
