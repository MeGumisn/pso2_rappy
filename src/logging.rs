use simple_logger::SimpleLogger;
use std::sync::Once;

static INIT: Once = Once::new();

pub(crate) fn init_logger() {
    INIT.call_once(|| {
        SimpleLogger::new().init().unwrap();
    });
}