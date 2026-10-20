use hopes::{app::App, systems::sync, tui};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if let [_, flag, dir, remote, branch] = args.as_slice()
        && flag == sync::FLUSH_ARG
    {
        sync::flush(std::path::Path::new(dir), remote, branch);
        return Ok(());
    }

    let mut terminal = tui::init()?;
    let mut app = App::new();
    let result = app.run(&mut terminal);

    tui::restore()?;
    app.shutdown_sync();
    result?;
    Ok(())
}
