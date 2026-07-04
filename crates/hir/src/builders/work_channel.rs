use crossbeam_channel::{Receiver, Sender};

pub struct WorkChannel<T> {
    sender: Option<Sender<T>>,
    recv: Receiver<T>,
}

impl<T> WorkChannel<T> {
    pub fn new() -> Self {
        let (sender, recv) = crossbeam_channel::unbounded();
        Self {
            sender: Some(sender),
            recv,
        }
    }
    pub fn send(&self, task: T) {
        self.sender.as_ref().unwrap().send(task).unwrap();
    }

    pub fn receiver(&self) -> &Receiver<T> {
        &self.recv
    }

    pub fn recv(&self) -> Option<T> {
        self.recv.recv().ok()
    }

    /// Drop the sender so that [`recv`] returns `None` once the channel is empty.
    pub fn close_sender(&mut self) {
        self.sender.take();
    }
}
