use chrono::Local;
use fern::{
    self,
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};

pub fn init() {
    let colors_level = ColoredLevelConfig::new()
        .trace(Color::BrightBlack)
        .debug(Color::BrightCyan)
        .info(Color::BrightBlue);

    let base_config = Dispatch::new();

    let file_config = Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{date}[{level}][{target}] {message}",
                level = colors_level.color(record.level()),
                date = Local::now().format("[%y-%m-%d %H:%M:%S]"),
                target = record.target(),
                message = message
            ))
        })
        .level(log::LevelFilter::Trace)
        .level_for("serenity", log::LevelFilter::Info)
        .level_for("tracing::span", log::LevelFilter::Info)
        .chain(fern::log_file("output.log").expect("could not open log file"));

    let stdout_config = Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{date}[{level}][{target}] {message}",
                level = colors_level.color(record.level()),
                date = Local::now().format("[%y-%m-%d %H:%M:%S]"),
                target = record.target(),
                message = message
            ))
        })
        .level(log::LevelFilter::Info)
        .level_for("serenity", log::LevelFilter::Warn)
        .level_for("tracing::span", log::LevelFilter::Warn)
        .level_for("ultron", log::LevelFilter::Debug)
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()
        .expect("unable to init logger");
}
