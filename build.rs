use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    let git_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    let status = Command::new("sh")
        .arg("-c")
        .arg("cd frontend && npm install && npm run build")
        .status();

    if std::env::var("SKIP_FRONTEND_BUILD").is_ok() {
        return;
    }

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            panic!("frontend build failed with status: {s}");
        }
        Err(e) => {
            panic!("could not run frontend build: {e}");
        }
    }
}
