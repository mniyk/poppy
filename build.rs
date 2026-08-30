fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        if let Err(err) = res.compile() {
            eprintln!("failed to embed icon: {err}");
        }
    }
}
