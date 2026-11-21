fn main() {
    // I comandi dell'app sono permessi anche alla dashboard remota (vedi capabilities/dashboard.json).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_info",
            "get_folder",
            "pick_folder",
            "file_exists",
            "save_file",
            "open_folder",
            "xml_info",
            "xml_pick",
            "xml_read",
            "xml_sent",
        ]),
    ))
    .expect("errore in tauri-build");
}
