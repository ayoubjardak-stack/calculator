// No console window behind the app on Windows.
#![windows_subsystem = "windows"]

fn main() -> iced::Result {
    calculator_app::run()
}
