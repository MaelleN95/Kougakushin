fn main() {
    // Recompile quand une migration est ajoutée ou modifiée : `sqlx::migrate!` les intègre au binaire.
    println!("cargo:rerun-if-changed=migrations");
    tauri_build::build()
}
