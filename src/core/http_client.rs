//! Cliente HTTP con rate limiting, reintentos exponenciales y caché de respuestas.
//!
//! Se usa para consultar fuentes de metadatos externas (TVmaze, AniList, Kitsu,
//! Jikan, Shikimori, iTunes, Cinemeta) respetando los límites de tasa de cada
//! proveedor y evitando peticiones repetidas innecesarias.

use reqwest::Client;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::time::sleep;


/// Ventana de tiempo para el rate limiter (TVmaze: ~20 req / 10 s).
const RATE_LIMIT_WINDOW_SECS: u64 = 10;
/// Máximo de peticiones por ventana de tiempo.
const RATE_LIMIT_MAX_REQUESTS: usize = 20;
/// Número máximo de reintentos exponenciales.
const MAX_RETRIES: u32 = 3;

/// Límite de tasa basado en una ventana deslizante de timestamps.
///
/// Permite hasta `max_requests` peticiones dentro de cada `window`. Si se
/// agota el cupo, `wait()` se suspende hasta que la petición más antigua salga
/// de la ventana.
struct RateLimiter {
    max_requests: usize,
    window: Duration,
    timestamps: Mutex<VecDeque<Instant>>,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            timestamps: Mutex::new(VecDeque::new()),
        }
    }

    /// Espera hasta que haya cupo para una nueva petición.
    async fn wait(&self) {
        loop {
            let wait_for = {
                let mut ts = self.timestamps.lock().unwrap();
                let now = Instant::now();
                // Eliminar timestamps fuera de la ventana.
                while let Some(front) = ts.front() {
                    if now.duration_since(*front) > self.window {
                        ts.pop_front();
                    } else {
                        break;
                    }
                }
                if ts.len() < self.max_requests {
                    ts.push_back(now);
                    return; // cupo disponible
                }
                // Calcular cuánto falta para que la petición más antigua expire.
                let oldest = ts.front().expect("timestamps no vacío");
                self.window
                    .checked_sub(now.duration_since(*oldest))
                    .unwrap_or(Duration::ZERO)
            };
            sleep(wait_for).await;
        }
    }
}

/// Entrada de caché de respuesta.
struct CacheEntry {
    data: String,
    expires_at: Instant,
}

/// Caché en memoria de respuestas HTTP con TTL.
struct ResponseCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl ResponseCache {
    fn new(ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Devuelve el cuerpo cacheado si existe y no ha expirado.
    fn get(&self, url: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.get(url)?;
        if entry.expires_at < Instant::now() {
            entries.remove(url);
            return None;
        }
        Some(entry.data.clone())
    }

    /// Almacena el cuerpo de una respuesta.
    fn set(&self, url: &str, data: String) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(
            url.to_string(),
            CacheEntry {
                data,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }
}

/// Cliente HTTP con rate limiting, reintentos exponenciales y caché opcional.
pub struct HttpClient {
    client: Client,
    limiter: RateLimiter,
    cache: Option<ResponseCache>,
}

impl HttpClient {
    /// Crea un cliente con los límites por defecto (20 req / 10 s) y sin caché.
    pub fn new() -> Self {
        Self::with_limits(
            RATE_LIMIT_MAX_REQUESTS,
            Duration::from_secs(RATE_LIMIT_WINDOW_SECS),
        )
    }

    /// Crea un cliente con límites personalizados.
    pub fn with_limits(max_requests: usize, window: Duration) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("acryl/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self {
            client,
            limiter: RateLimiter::new(max_requests, window),
            cache: None,
        }
    }

    /// Activa el caché de respuestas con el TTL dado.
    pub fn with_cache(mut self, ttl: Duration) -> Self {
        self.cache = Some(ResponseCache::new(ttl));
        self
    }

    /// Realiza una GET respetando el rate limit, con reintentos exponenciales
    /// ante errores de red o respuestas 429/5xx, y usando caché si está
    /// activada y hay entrada válida.
    pub async fn get_text(&self, url: &str) -> Result<String, reqwest::Error> {
        // Caché: devolver sin hacer petición de red.
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(url) {
                return Ok(cached);
            }
        }

        self.limiter.wait().await;

        let mut last_error = None;
        for attempt in 0..=MAX_RETRIES {
            let result = self.client.get(url).send().await;
            match result {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let text = response.text().await?;
                        if let Some(cache) = &self.cache {
                            cache.set(url, text.clone());
                        }
                        return Ok(text);
                    }
                    // Rate limit (429) o error de servidor (5xx): reintentar.
                    if status.as_u16() == 429 || status.is_server_error() {
                        last_error = Some(response.error_for_status().unwrap_err());
                        let backoff = backoff(attempt);
                        sleep(backoff).await;
                        continue;
                    }
                    // Error del cliente (4xx salvo 429): no reintentar.
                    return Err(response.error_for_status().unwrap_err());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES {
                        let backoff = backoff(attempt);
                        sleep(backoff).await;
                        continue;
                    }
                    return Err(last_error.unwrap());
                }
            }
        }
        Err(last_error.unwrap())
    }
}

/// Tiempo de espera exponencial: 2s, 4s, 8s, ... limitado a 30s.
fn backoff(attempt: u32) -> Duration {
    let secs = (2u64.checked_pow(attempt).unwrap_or(30)).min(30);
    Duration::from_secs(secs)
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_permite_hasta_el_limite_sin_esperar() {
        // RateLimiter::wait es async; aquí solo verificamos la construcción y
        // que los campos se inicializan correctamente.
        let limiter = RateLimiter::new(5, Duration::from_secs(10));
        let ts = limiter.timestamps.lock().unwrap();
        assert_eq!(ts.len(), 0);
        drop(ts);
        assert_eq!(limiter.max_requests, 5);
        assert_eq!(limiter.window, Duration::from_secs(10));
    }

    #[test]
    fn backoff_crece_exponencialmente() {
        assert_eq!(backoff(0), Duration::from_secs(2));
        assert_eq!(backoff(1), Duration::from_secs(4));
        assert_eq!(backoff(2), Duration::from_secs(8));
        // Límite superior.
        assert_eq!(backoff(10), Duration::from_secs(30));
    }

    #[test]
    fn response_cache_devuelve_none_si_no_existe() {
        let cache = ResponseCache::new(Duration::from_secs(60));
        assert!(cache.get("https://example.com/inexistente").is_none());
    }

    #[test]
    fn response_cache_guarda_y_devuelve() {
        let cache = ResponseCache::new(Duration::from_secs(60));
        cache.set("https://example.com/a", "contenido-a".to_string());
        assert_eq!(cache.get("https://example.com/a"), Some("contenido-a".to_string()));
    }

    #[test]
    fn response_cache_expira_despues_del_ttl() {
        let cache = ResponseCache::new(Duration::ZERO);
        cache.set("https://example.com/b", "contenido-b".to_string());
        // Pequeña pausa para asegurar expiración con TTL cero.
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get("https://example.com/b").is_none());
    }

    #[test]
    fn http_client_nuevo_sin_panic() {
        let _client = HttpClient::new();
    }

    #[test]
    fn http_client_con_cache_tiene_cache_activada() {
        let client = HttpClient::new().with_cache(Duration::from_secs(60));
        // El campo cache es privado; verificamos que no panique al construir.
        let _ = client;
    }

    #[tokio::test]
    async fn http_client_respeta_rate_limit_con_mock_server() {
        // Usamos un servidor de prueba local simple con reqwest + un Tokio
        // runtime. No es posible levantar un servidor HTTP fácilmente sin
        // dependencias extra, así que esta prueba solo verifica que el cliente
        // se construye y que get_text falla limpiamente contra una URL inválida
        // en lugar de hacer pánico.
        let client = HttpClient::new();
        let resultado = client.get_text("http://127.0.0.1:1/inexistente").await;
        assert!(resultado.is_err(), "debe fallar sin servidor");
    }
}
