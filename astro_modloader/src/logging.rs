use std::fs;
use std::io::{prelude::*, BufWriter};
use std::sync::{Mutex, OnceLock};

use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

#[derive(Debug)]
struct SimpleLogger {
    file: Mutex<BufWriter<fs::File>>,
}

impl SimpleLogger {
    fn new(file: fs::File) -> Self {
        SimpleLogger {
            file: Mutex::new(BufWriter::new(file)),
        }
    }

    fn lock<T>(&self, f: impl FnOnce(&mut BufWriter<fs::File>) -> T) -> T {
        let mut guard = match self.file.lock() {
            Ok(guard) => guard,
            Err(err) => err.into_inner(),
        };

        f(&mut guard)
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let file_path = match record.file() {
                Some(path) => match path.split_once(".cargo") {
                    // cut down to only lib name
                    Some((_, path)) => &path[42..],
                    None => path,
                },
                None => "<unknown>",
            };

            // if it's from a dependency only log debug and above, else everything
            if !record.file().unwrap_or("").contains(".cargo") || record.level() <= Level::Debug {
                let level = match record.level() {
                    Level::Error => "ERROR".red(),
                    Level::Warn => "WARN".yellow(),
                    Level::Info => "INFO".green(),
                    Level::Debug => "DEBUG".cyan(),
                    Level::Trace => "TRACE".blue(),
                };

                println!(
                    "{}{level:<5} {file_path}:{}{} {}",
                    "[".truecolor(100, 100, 100),
                    record.line().unwrap_or(0),
                    "]".truecolor(100, 100, 100),
                    record.args()
                );
            }

            let level = match record.level() {
                Level::Error => "ERROR",
                Level::Warn => "WARN",
                Level::Info => "INFO",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            };

            self.lock(|file| {
                writeln!(
                    file,
                    "[{level:<5} {file_path}:{}] {}",
                    record.line().unwrap_or(0),
                    record.args()
                )
            })
            .unwrap();
        }
    }

    fn flush(&self) {
        self.lock(|file| file.flush()).unwrap()
    }
}

fn get_logger() -> &'static SimpleLogger {
    static LOGGER: OnceLock<SimpleLogger> = OnceLock::new();
    LOGGER.get_or_init(|| {
        SimpleLogger::new(
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open("modloader_log.txt")
                .unwrap(),
        )
    })
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(get_logger()).map(|()| log::set_max_level(LevelFilter::Trace))
}

pub fn flush() {
    get_logger().flush()
}
