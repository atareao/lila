use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u128 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .expect("El sistema tiene tiempo negativo")
        .as_millis()
}

/// Momento actual en milisegundos desde la época Unix.
///
/// Convierte el valor de [`now`] (`u128`) a `i64` saturando a `i64::MAX` si no
/// cabe, de modo que nunca entra en pánico por conversión.
pub fn now_millis() -> i64 {
    i64::try_from(now()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [TDD-RED-B3-02] El helper devuelve una marca temporal positiva.
    #[test]
    fn now_millis_returns_positive_timestamp() {
        assert!(now_millis() > 0);
    }
}
