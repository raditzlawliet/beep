mod tui;

fn main() {
    if let Err(e) = color_eyre::install() {
        eprintln!("Warning: {}", e);
    }

    if let Err(e) = ratatui::run(tui::app) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
