from pathlib import Path
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]


def git_dependencies(table):
    return sorted(name for name, spec in table.items() if isinstance(spec, dict) and "git" in spec)


class DependencySourceTests(unittest.TestCase):
    """Cranamp builds only from published crates, including Cranpose and its forks."""

    def test_the_manifest_names_no_git_dependency(self):
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
        tables = {"dependencies": manifest.get("dependencies", {})}
        tables["dev-dependencies"] = manifest.get("dev-dependencies", {})
        tables["build-dependencies"] = manifest.get("build-dependencies", {})
        for target, sections in manifest.get("target", {}).items():
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                tables[f"target.{target}.{section}"] = sections.get(section, {})
        for registry, table in manifest.get("patch", {}).items():
            tables[f"patch.{registry}"] = table
        found = {name: git_dependencies(table) for name, table in tables.items()}
        self.assertEqual({name: deps for name, deps in found.items() if deps}, {})

    def test_the_lockfile_resolves_no_package_from_git(self):
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        from_git = sorted(
            f"{package['name']} {package['version']}"
            for package in lock["package"]
            if package.get("source", "").startswith("git+")
        )
        self.assertEqual(from_git, [])


if __name__ == "__main__":
    unittest.main()
