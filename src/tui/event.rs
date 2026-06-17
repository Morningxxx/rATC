use crossterm::event::{self, Event, KeyEvent};
use std::time::{Duration, Instant};

pub enum Message {
    Key(KeyEvent),
    Tick,
}

/// Poll for a terminal event with a tick timeout.
pub fn poll(tick_ms: u64) -> Option<Message> {
    let deadline = Instant::now() + Duration::from_millis(tick_ms);
    while Instant::now() < deadline {
        if event::poll(Duration::from_millis(50)).ok()? {
            if let Event::Key(k) = event::read().ok()? {
                return Some(Message::Key(k));
            }
        }
    }
    Some(Message::Tick)
}
