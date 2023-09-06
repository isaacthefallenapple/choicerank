use std::collections::HashMap;

use sqlx::PgPool;
use tide::sse;
use tokio::sync;

pub type Request = tide::Request<State>;

#[derive(Clone, Debug)]
pub struct State {
    db: PgPool,
    sse: sync::mpsc::Sender<SseEvent>,
}

impl State {
    pub fn new(db: PgPool, sse_sender: sync::mpsc::Sender<SseEvent>) -> Self {
        Self {
            db,
            sse: sse_sender,
        }
    }

    pub fn db(&self) -> &PgPool {
        &self.db
    }

    pub async fn register_sse_sender(&self, id: i32, sender: sse::Sender) {
        let _ = inspect_err(self.sse.send(SseEvent::RegisterSender { id, sender }).await);
    }

    pub async fn send_to(&self, id: i32, name: &str, value: &str) {
        let _ = inspect_err(
            self.sse
                .send(SseEvent::BroadcastEvent {
                    receiver: id,
                    name: name.to_string(),
                    data: value.to_string(),
                })
                .await,
        );
    }
}

pub async fn handle_sse(mut rx: sync::mpsc::Receiver<SseEvent>) {
    fn is_closed(err: &std::io::Error) -> bool {
        err.kind() == std::io::ErrorKind::ConnectionAborted
    }

    let mut map: HashMap<i32, Vec<Option<sse::Sender>>> = HashMap::new();

    while let Some(event) = rx.recv().await {
        match event {
            SseEvent::RegisterSender { id, sender } => {
                map.entry(id).or_default().push(Some(sender));
            }
            SseEvent::BroadcastEvent {
                ref receiver,
                name,
                data,
            } => {
                let Some(receivers) = map.get_mut(receiver) else {
                    continue;
                };

                for slot in receivers.iter_mut() {
                    let Some(receiver) = slot.take() else { continue; };
                    let is_closed = match receiver.send(&name, &data, None).await {
                        Err(ref err) => is_closed(err),
                        _ => false,
                    };
                    if !is_closed {
                        *slot = Some(receiver);
                    }
                }

                receivers.retain(Option::is_some);
            }
        }
    }
}

pub enum SseEvent {
    RegisterSender {
        id: i32,
        sender: sse::Sender,
    },
    BroadcastEvent {
        receiver: i32,
        name: String,
        data: String,
    },
}

fn inspect_err<T, E: std::fmt::Debug>(res: Result<T, E>) -> Result<T, E> {
    if let Err(e) = res.as_ref() {
        eprintln!("{e:?}");
    }

    res
}
