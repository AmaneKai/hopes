use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::{
    io,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::Instant,
};

pub enum Event {
    Key(KeyEvent),
    Resize,
    Wake,
}

pub struct EventHandler {
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let reader = tx.clone();
        thread::spawn(move || {
            while let Ok(event) = event::read() {
                let event = match event {
                    CrosstermEvent::Key(key) => Event::Key(key),
                    CrosstermEvent::Resize(..) => Event::Resize,
                    _ => continue,
                };
                if reader.send(event).is_err() {
                    break;
                }
            }
        });
        Self { tx, rx }
    }

    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    pub fn next(&self, deadline: Option<Instant>) -> io::Result<Option<Event>> {
        let received = match deadline {
            Some(at) => self
                .rx
                .recv_timeout(at.saturating_duration_since(Instant::now())),
            None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(event) => Ok(Some(event)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(io::ErrorKind::UnexpectedEof.into()),
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
