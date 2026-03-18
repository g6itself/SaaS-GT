// ── Rate limiter in-process (token bucket simplifié) ─────────────────────────
//
// Utilisation :
//   let limiter = web::Data::new(RateLimiter::new(5, 60)); // 5 req / 60s
//   App::new().app_data(limiter.clone())
//
//   Dans un handler :
//   if !limiter.check(ip_str) { return HttpResponse::TooManyRequests()... }

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    state: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Retourne true si la requête est autorisée, false si le quota est dépassé.
    pub fn check(&self, key: &str) -> bool {
        let mut map = match self.state.lock() {
            Ok(m) => m,
            Err(_) => return true, // En cas de poison, laisser passer (fail-open)
        };

        let now = Instant::now();

        // Nettoyage périodique des entrées expirées (1 sur 100 appels en moyenne)
        if map.len() > 10_000 {
            map.retain(|_, (_, ts)| now.duration_since(*ts) < self.window);
        }

        let entry = map.entry(key.to_string()).or_insert((0, now));

        // Réinitialiser le compteur si la fenêtre est expirée
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }

        if entry.0 >= self.max_requests {
            return false;
        }

        entry.0 += 1;
        true
    }
}

/// Extrait l'IP réelle depuis une requête actix-web.
/// Respecte X-Forwarded-For si présent (derrière un proxy/nginx).
pub fn client_ip(req: &actix_web::HttpRequest) -> String {
    req.headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            req.peer_addr()
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}
