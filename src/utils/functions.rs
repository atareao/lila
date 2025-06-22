use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u128 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .expect("El sistema tiene tiempo negativo")
        .as_millis()
}
