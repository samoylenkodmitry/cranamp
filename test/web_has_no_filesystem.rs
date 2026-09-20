use std::path::{Path, PathBuf};
const HAZARD: &str = "std::env::temp_dir";
const GUARD: &str = "#[cfg(not(target_arch = \"wasm32\"))]";
fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .expect("readable source tree")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}
fn inside_test_module(lines: &[&str]) -> Vec<bool> {
    let mut flag = vec![false; lines.len()];
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "#[cfg(test)]" {
            let mut open = i + 1;
            while open < lines.len() && open <= i + 3 && !lines[open].contains('{') {
                open += 1;
            }
            if open < lines.len() && lines[open].contains('{') {
                let mut depth = 0i32;
                let mut end = open;
                loop {
                    depth += lines[end].matches('{').count() as i32;
                    depth -= lines[end].matches('}').count() as i32;
                    flag[end] = true;
                    if depth <= 0 || end + 1 >= lines.len() {
                        break;
                    }
                    end += 1;
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    flag
}
#[test]
fn every_temp_dir_call_sits_behind_a_cfg_the_web_build_does_not_take() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    rust_sources(&root, &mut sources);
    assert!(
        sources.len() > 3,
        "expected to walk the crate sources, found {}",
        sources.len()
    );
    let mut checked = 0;
    for file in &sources {
        let text = std::fs::read_to_string(file).expect("readable source file");
        let lines: Vec<&str> = text.lines().collect();
        let in_test = inside_test_module(&lines);
        for (n, line) in lines.iter().enumerate() {
            if in_test[n] || line.trim_start().starts_with("//") || !line.contains(HAZARD) {
                continue;
            }
            checked += 1;
            let guarded = (n.saturating_sub(4)..=n).any(|m| lines[m].contains(GUARD));
            assert!(
                guarded,
                "{}:{} calls {HAZARD}(), which panics on wasm32 with \"no \
                 filesystem on this platform\". Put it behind {GUARD} and give \
                 the web build its own branch: a bare file name the browser \
                 places, or a path it never touches.\n    {}",
                file.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(file)
                    .display(),
                n + 1,
                line.trim()
            );
        }
    }
    assert!(
        checked >= 2,
        "expected to find the guarded desktop calls; found {checked}"
    );
}
