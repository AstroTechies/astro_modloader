use winres::WindowsResource;

fn main() {
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .unwrap();
    }
}
