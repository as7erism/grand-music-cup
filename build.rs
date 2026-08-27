fn main() {
    std::process::Command::new("npx")
        .args([
            "@tailwindcss/cli",
            "-i",
            "./tailwind.css",
            "-o",
            "./assets/style.css",
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
