pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self { should_quit: false };
        app
    }
}
