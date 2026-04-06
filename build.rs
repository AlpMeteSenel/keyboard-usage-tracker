use std::fs;
use std::path::Path;

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("keyboard_logo.ico");
        res.compile().expect("Failed to compile Windows resources");
    }

    // Assemble dashboard HTML from base template + plugin files
    assemble_dashboard();
}

fn assemble_dashboard() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let base_path = "dashboard_base.html";
    let plugins_dir = "plugins";
    let output_path = Path::new(&out_dir).join("dashboard.html");

    // Tell Cargo to re-run if any of these change
    println!("cargo::rerun-if-changed={}", base_path);
    println!("cargo::rerun-if-changed={}", plugins_dir);

    let base_html = fs::read_to_string(base_path).expect("Failed to read dashboard_base.html");

    // Collect all .js plugin files, sorted alphabetically for deterministic order
    let mut plugin_files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "js") {
                plugin_files.push(path.display().to_string());
            }
        }
    }
    plugin_files.sort();

    // Also rerun if individual plugin files change
    for f in &plugin_files {
        println!("cargo::rerun-if-changed={}", f);
    }

    // Read and concatenate all plugin scripts
    let mut plugin_scripts = String::new();
    for f in &plugin_files {
        let content =
            fs::read_to_string(f).unwrap_or_else(|_| panic!("Failed to read plugin file: {}", f));
        plugin_scripts.push_str(&format!(
            "<script>\n// — Plugin: {} —\n{}\n</script>\n",
            f, content
        ));
    }

    // Inject plugins at the marker
    let assembled = base_html.replace("<!-- __PLUGINS__ -->", &plugin_scripts);

    fs::write(&output_path, assembled).expect("Failed to write assembled dashboard.html");
}
