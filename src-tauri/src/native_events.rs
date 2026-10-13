use crate::models::TransportEvent;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const MAX_PENDING_BYTES: usize = 2 * 1024 * 1024;
const MAX_PENDING_EVENTS: usize = 1024;

pub(crate) struct TimedEvent {
    pub event: TransportEvent,
    pub timestamp: String,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Counters {
    pub sent: u64,
    pub received: u64,
    pub trimmed: u64,
}

#[derive(Default)]
struct Pending {
    events: VecDeque<TimedEvent>,
    bytes: usize,
    counters: Counters,
}

#[derive(Clone, Default)]
pub(crate) struct Inbox(Arc<Mutex<Pending>>);

impl Inbox {
    pub fn push(&self, event: TransportEvent) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if event.kind == "data" {
            if event.direction == "received" {
                pending.counters.received += event.bytes.len() as u64;
            } else {
                pending.counters.sent += event.bytes.len() as u64;
            }
        }
        pending.bytes += event.bytes.len();
        pending.events.push_back(TimedEvent {
            event,
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
        });
        while pending.bytes > MAX_PENDING_BYTES || pending.events.len() > MAX_PENDING_EVENTS {
            let index = pending
                .events
                .iter()
                .position(|item| item.event.kind == "data")
                .unwrap_or(0);
            if let Some(item) = pending.events.remove(index) {
                pending.bytes -= item.event.bytes.len();
                pending.counters.trimmed += item.event.bytes.len() as u64;
            }
        }
    }

    pub fn pop(&self) -> Option<TimedEvent> {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let event = pending.events.pop_front()?;
        pending.bytes -= event.event.bytes.len();
        Some(event)
    }

    pub fn counters(&self) -> Counters {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .counters
    }

    pub fn is_empty(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .events
            .is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_display_backlog_without_losing_counters_or_connection_state() {
        let inbox = Inbox::default();
        inbox.push(TransportEvent {
            kind: "connected".into(),
            direction: "system".into(),
            bytes: vec![],
            message: "ready".into(),
        });
        for _ in 0..2000 {
            inbox.push(TransportEvent {
                kind: "data".into(),
                direction: "received".into(),
                bytes: vec![0; 4096],
                message: String::new(),
            });
        }
        assert_eq!(inbox.counters().received, 2000 * 4096);
        assert!(inbox.counters().trimmed > 0);
        assert_eq!(inbox.pop().unwrap().event.kind, "connected");
        let pending = inbox.0.lock().unwrap();
        assert!(pending.bytes <= MAX_PENDING_BYTES);
        assert!(pending.events.len() <= MAX_PENDING_EVENTS);
    }
}
