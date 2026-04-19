use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");

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
