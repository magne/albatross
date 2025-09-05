use std::{collections::{HashSet, VecDeque}, sync::Arc, time::{Duration, Instant}};
use axum::{
    extract::{State, Query},
    response::Response,
    http::{HeaderMap, StatusCode},
};
use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message, CloseFrame};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::interval};
use tracing::{info, warn, error};
use crate::{AppState, application::middleware::AuthenticatedUser};

/// Query params allow fallback api_key for browser dev (`/api/ws?api_key=...`)
#[derive(Deserialize)]
pub struct WsQuery {
    pub api_key: Option<String>,
}

#[derive(Serialize)]
struct ErrorFrame {
    r#type: &'static str,
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct HeartbeatFrame {
    r#type: &'static str,
    ts: String,
}

#[derive(Serialize)]
struct AckFrame {
    r#type: &'static str,
    action: &'static str,
    channels: Vec<String>,
    accepted: Vec<String>,
    rejected: Vec<String>,
}

#[derive(Serialize)]
struct PongFrame {
    r#type: &'static str,
    id: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Inbound {
    #[serde(rename = "subscribe")]
    Subscribe { channels: Vec<String> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { channels: Vec<String> },
    Ping { id: Option<String> },
    #[serde(other)]
    Unknown,
}

#[derive(Default)]
struct RateLimiter {
    events: VecDeque<Instant>,
    max: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max: usize, window: Duration) -> Self {
        Self { events: VecDeque::new(), max, window }
    }
    fn record(&mut self) -> bool {
        let now = Instant::now();
        while let Some(front) = self.events.front() {
            if now.duration_since(*front) > self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }
        self.events.push_back(now);
        self.events.len() <= self.max
    }
}

/// Public handler registered at `/api/ws`
pub async fn ws_handler(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    // Extract API key from header or query
    let api_key = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .or_else(|| q.api_key.clone());

    let Some(key) = api_key else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    // Authenticate via cache (same model as middleware)
    let ctx = match app_state.cache.get(&key).await {
        Ok(Some(bytes)) => {
            match serde_json::from_slice::<AuthenticatedUser>(&bytes) {
                Ok(c) => c,
                Err(_) => return Err(StatusCode::UNAUTHORIZED),
            }
        }
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // NOTE: For now, if Redis client is absent we still accept connection but only send heartbeat frames.
    let redis_client = app_state.redis_client.clone();

    Ok(ws.on_upgrade(move |socket| websocket_connection(socket, ctx, redis_client)))
}

async fn websocket_connection(
    socket: WebSocket,
    ctx: AuthenticatedUser,
    redis_client: Option<redis::Client>,
) {
    let conn_id = uuid::Uuid::new_v4();
    info!(%conn_id, user_id = %ctx.user_id, "WebSocket connection established");

    // Shared subscription state (placeholder – real subscriptions added later)
    let subscriptions = Arc::new(Mutex::new(HashSet::<String>::new()));
    // Rate limiter
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(10, Duration::from_secs(10))));
    // Activity tracking
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    // Baseline auto-subscriptions (store only; actual Redis subscription deferred until full impl)
    {
        let mut subs = subscriptions.lock().await;
        subs.insert(format!("user:{}:updates", ctx.user_id));
        subs.insert(format!("user:{}:apikeys", ctx.user_id));
        if let Some(tid) = &ctx.tenant_id {
            subs.insert(format!("tenant:{}:updates", tid));
        }
    }

    // Split socket and wrap sender in Arc<Mutex<..>> so we can use in multiple tasks
    let (sender_raw, mut receiver) = socket.split();
    let sender_arc = Arc::new(Mutex::new(sender_raw));

    let hb_last_activity = last_activity.clone();
    let mut hb_interval = interval(Duration::from_secs(30));
    {
        let sender_clone = Arc::clone(&sender_arc);
        tokio::spawn(async move {
            loop {
                hb_interval.tick().await;
                let ts = chrono::Utc::now().to_rfc3339();
                let frame = HeartbeatFrame { r#type: "heartbeat", ts };
                let json = match serde_json::to_string(&frame) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to serialize heartbeat: {}", e);
                        continue;
                    }
                };
                if sender_clone.lock().await.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
                // Update last activity (outbound)
                {
                    let mut la = hb_last_activity.lock().await;
                    *la = Instant::now();
                }
            }
        });
    }

    // Redis forward loop (subscribe baseline)
    if let Some(client) = redis_client {
        match client.get_async_pubsub().await {
            Ok(mut pubsub) => {
                {
                    let subs = subscriptions.lock().await;
                    for ch in subs.iter() {
                        if let Err(e) = pubsub.subscribe(ch).await {
                            warn!(%conn_id, channel=%ch, "Failed subscribing to channel: {}", e);
                        }
                    }
                }
                let sender_clone = Arc::clone(&sender_arc);
                let subs_clone = Arc::clone(&subscriptions);
                tokio::spawn(async move {
                    let mut stream = pubsub.into_on_message();
                    while let Some(msg) = stream.next().await {
                        let channel: String = match msg.get_channel() {
                            Ok(c) => c,
                            Err(_) => continue,
                        };
                        // Only forward if still subscribed
                        if !subs_clone.lock().await.contains(&channel) {
                            continue;
                        }
                        let payload: Vec<u8> = match msg.get_payload() {
                            Ok(p) => p,
                            Err(e) => {
                                warn!("Failed to read redis payload: {}", e);
                                continue;
                            }
                        };
                        // Assume UTF-8 JSON payload; wrap
                        if let Ok(txt) = String::from_utf8(payload) {
                            let frame = serde_json::json!({
                                "type": "event",
                                "channel": channel,
                                "payload": serde_json::from_str::<serde_json::Value>(&txt).unwrap_or(serde_json::json!({"raw": txt}))
                            });
                            if let Ok(out) = serde_json::to_string(&frame)
                                && sender_clone.lock().await.send(Message::Text(out.into())).await.is_err() {
                                    break;
                                }
                        }
                    }
                    info!(%conn_id, "Redis forward loop ended");
                });
            }
            Err(e) => {
                warn!(%conn_id, "Failed to create Redis pubsub: {}", e);
            }
        }
    } else {
        warn!(%conn_id, "Redis client not configured; real-time events disabled.");
    }

    // Reader loop
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(txt) => {
                *last_activity.lock().await = Instant::now();
                if txt.len() > 32 * 1024 {
                    let _ = send_error(&sender_arc, "invalid_message", "Message too large").await;
                    continue;
                }
                // Rate limit
                {
                    let mut rl = rate_limiter.lock().await;
                    if !rl.record() {
                        let _ = send_error(&sender_arc, "rate_limited", "Too many control messages").await;
                        continue;
                    }
                }
                // Parse inbound message
                let inbound: Result<Inbound, _> = serde_json::from_str(&txt);
                match inbound {
                    Ok(Inbound::Subscribe { channels }) => {
                        let mut accepted = Vec::new();
                        let mut rejected = Vec::new();
                        // Subscribe each valid channel
                        for ch in channels.iter() {
                            if validate_channel(ch, &ctx) {
                                let mut subs = subscriptions.lock().await;
                                if subs.insert(ch.clone()) {
                                    accepted.push(ch.clone());
                                }
                            } else {
                                rejected.push(ch.clone());
                            }
                        }
                        // ACK
                        let frame = AckFrame {
                            r#type: "ack",
                            action: "subscribe",
                            channels,
                            accepted: accepted.clone(),
                            rejected: rejected.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&frame)
                            && sender_arc.lock().await.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                    }
                    Ok(Inbound::Unsubscribe { channels }) => {
                        let mut removed = Vec::new();
                        let mut missing = Vec::new();
                        for ch in channels.iter() {
                            let mut subs = subscriptions.lock().await;
                            if subs.remove(ch) {
                                removed.push(ch.clone());
                            } else {
                                missing.push(ch.clone());
                            }
                        }
                        let frame = serde_json::json!({
                            "type":"ack",
                            "action":"unsubscribe",
                            "channels": channels,
                            "removed": removed,
                            "missing": missing
                        });
                        if sender_arc.lock().await.send(Message::Text(frame.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(Inbound::Ping { id }) => {
                        let frame = PongFrame { r#type: "pong", id };
                        if let Ok(json) = serde_json::to_string(&frame)
                            && sender_arc.lock().await.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                    }
                    Ok(Inbound::Unknown) | Err(_) => {
                        let _ = send_error(&sender_arc, "invalid_message", "Unrecognized message").await;
                    }
                }
            }
            Message::Binary(_) => {
                let _ = send_error(&sender_arc, "invalid_message", "Binary frames not supported").await;
            }
            Message::Ping(data) => {
                if sender_arc.lock().await.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => {
                *last_activity.lock().await = Instant::now();
            }
            Message::Close(_) => {
                info!(%conn_id, "Client closed connection");
                break;
            }
        }

        // Idle timeout (90s)
        if last_activity.lock().await.elapsed() > Duration::from_secs(90) {
            info!(%conn_id, "Idle timeout reached; closing");
            let _ = sender_arc.lock().await.send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "Idle timeout".into(),
            }))).await;
            break;
        }
    }

    info!(%conn_id, "WebSocket connection terminated");
}

async fn send_error(
    sender: &Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
    code: &'static str,
    message: &str
) -> Result<(), ()> {
    let frame = ErrorFrame { r#type: "error", code, message: message.to_string() };
    if let Ok(json) = serde_json::to_string(&frame)
        && sender.lock().await.send(Message::Text(json.into())).await.is_err() {
            return Err(());
        }
    Ok(())
}

// Channel validation logic
pub(crate) fn validate_channel(channel: &str, ctx: &AuthenticatedUser) -> bool {
    // Patterns: user:{id}:updates | user:{id}:apikeys | tenant:{tid}:updates
    let parts: Vec<&str> = channel.split(':').collect();
    if parts.len() != 3 {
        return false;
    }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::middleware::AuthenticatedUser;

    #[allow(dead_code)]
    fn ctx(user_id: &str, tenant_id: Option<&str>) -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: user_id.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            role: "TenantAdmin".to_string(),
        }
    }

    #[allow(dead_code)]
    #[test]
    fn validate_own_user_channels() {
        let c = ctx("u1", Some("t1"));
        assert!(validate_channel("user:u1:updates", &c));
        assert!(validate_channel("user:u1:apikeys", &c));
        assert!(!validate_channel("user:u2:updates", &c));
    }

    #[allow(dead_code)]
    #[test]
    fn validate_tenant_channel() {
        let c = ctx("u1", Some("t1"));
        assert!(validate_channel("tenant:t1:updates", &c));
        assert!(!validate_channel("tenant:t2:updates", &c));
    }

    #[allow(dead_code)]
    #[test]
    fn validate_pilot_cannot_other_user() {
        let mut c = ctx("u1", Some("t1"));
        c.role = "Pilot".to_string();
        assert!(validate_channel("user:u1:updates", &c));
        assert!(!validate_channel("user:u2:updates", &c));
    }

    #[allow(dead_code)]
    #[test]
    fn invalid_patterns_rejected() {
        let c = ctx("u1", Some("t1"));
        assert!(!validate_channel("user:u1", &c));
        assert!(!validate_channel("users:u1:updates", &c));
        assert!(!validate_channel("tenant::updates", &c));
        assert!(!validate_channel("tenant:t1:other", &c));
    }

    #[allow(dead_code)]
    #[tokio::test]
    async fn rate_limiter_window() {
        let mut rl = RateLimiter::new(3, Duration::from_millis(50));
        assert!(rl.record());
        assert!(rl.record());
        assert!(rl.record());
        assert!(!rl.record()); // 4th within window fails
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(rl.record()); // window expired
    }
}
    match (parts[0], parts[2]) {
        ("user", "updates") | ("user", "apikeys") => {
            // Only own user id allowed
            parts[1] == ctx.user_id
        }
        ("tenant", "updates") => {
            if let Some(tid) = &ctx.tenant_id {
                parts[1] == tid
            } else {
                false
            }
        }
        _ => false,
    }
}
