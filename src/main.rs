mod changes;
mod diff;
mod specs;
mod tui;
mod vfs;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tui::run()
}
