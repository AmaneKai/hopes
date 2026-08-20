use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::{io, time::Duration};

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    pub fn next(&self) -> io::Result<Event> {
        if event::poll(self.tick_rate)?
            && let CrosstermEvent::Key(key) = event::read()?
        {
            return Ok(Event::Key(key));
        }
        Ok(Event::Tick)
    }
}
