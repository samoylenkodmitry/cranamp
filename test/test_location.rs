use std::path::Path;

fn visit_sources(directory: &Path, violations: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).expect("readable source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            visit_sources(&path, violations);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let source = std::fs::read_to_string(&path).expect("Rust source");
            let lines: Vec<_> = source.lines().collect();
            for (line, text) in source.lines().enumerate() {
                if text.trim_start().starts_with("#[test]")
                    || text.trim_start().starts_with("#[tokio::test")
                {
                    violations.push(format!(
                        "{}:{}: tests belong under /test/, not in implementation files",
                        path.display(),
                        line + 1,
                    ));
                }
                if text.trim() == "#[cfg(test)]" {
                    let mut declaration = lines[line + 1..]
                        .iter()
                        .map(|text| text.trim())
                        .filter(|text| !text.is_empty());
                    let external_path = declaration.next().unwrap_or_default();
                    let module = declaration.next().unwrap_or_default();
                    if !(external_path.starts_with("#[path = ")
                        && external_path.contains("/test/")
                        && module.contains("mod ")
                        && module.ends_with(';')
                        && !module.contains('{'))
                    {
                        violations.push(format!(
                            "{}:{}: test-only code must be an external module under /test/",
                            path.display(),
                            line + 1,
                        ));
                    }
                }
            }
        }
    }
}

#[test]
fn implementation_files_do_not_contain_tests() {
    let mut violations = Vec::new();
    visit_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut violations,
    );
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}
