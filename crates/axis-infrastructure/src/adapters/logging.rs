use std::path::Path;

pub fn setup_logger(log_root: &Path, app_name: &str) -> Result<(), fern::InitError> {
    std::fs::create_dir_all(log_root).ok();

    let timestamp = chrono::Local::now().format("%Y-%m-%d");
    let log_file = log_root.join(format!("{}-{}.log", app_name, timestamp));

    let file_dispatcher = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(&log_file)?);

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(file_dispatcher);

    if let Ok(lvl) = std::env::var("RUST_LOG")
        && let Ok(parsed) = lvl.parse()
    {
        dispatch = dispatch.level(parsed);
    }

    dispatch.apply()?;
    Ok(())
}
