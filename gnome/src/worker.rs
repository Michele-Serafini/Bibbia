//! One sleeping worker owns SQLite. GTK never waits for a search or disk write.
use bible_core::Engine;
use futures_channel::oneshot;
use serde_json::Value;
use std::{cell::RefCell, path::PathBuf, sync::mpsc, thread};

type Reply = Result<Value, String>;
enum Message {
    Request(Value, oneshot::Sender<Reply>),
    Post(Value),
    Stop,
}

pub struct Worker {
    sender: mpsc::Sender<Message>,
    thread: RefCell<Option<thread::JoinHandle<()>>>,
}
impl Worker {
    pub fn new(directory: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("bibbia-core".into())
            .spawn(move || {
                #[cfg(unix)]
                let setup = {
                    use std::os::unix::fs::DirBuilderExt;
                    std::fs::DirBuilder::new()
                        .recursive(true)
                        .mode(0o700)
                        .create(&directory)
                        .map_err(|e| e.to_string())
                };
                #[cfg(not(unix))]
                let setup = std::fs::create_dir_all(&directory).map_err(|e| e.to_string());
                let mut engine = setup.and_then(|_| Engine::open(&directory));
                for message in receiver {
                    let run = |engine: &mut Result<Engine, String>, request| match engine {
                        Ok(e) => e.request(request),
                        Err(e) => Err(e.clone()),
                    };
                    match message {
                        Message::Request(request, reply) => {
                            let _ = reply.send(run(&mut engine, request));
                        }
                        Message::Post(request) => {
                            if let Err(e) = run(&mut engine, request) {
                                eprintln!("Bibbia: {e}");
                            }
                        }
                        Message::Stop => break,
                    }
                }
                // Engine/SQLite close here, including the WAL, before shutdown joins.
            })
            .expect("could not start the Bible worker");
        Self {
            sender,
            thread: RefCell::new(Some(thread)),
        }
    }
    pub async fn call(&self, request: Value) -> Reply {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Message::Request(request, tx))
            .map_err(|_| "Motore chiuso".to_owned())?;
        rx.await.map_err(|_| "Motore chiuso".to_owned())?
    }
    pub fn post(&self, request: Value) {
        let _ = self.sender.send(Message::Post(request));
    }
    pub fn shutdown(&self) {
        if let Some(thread) = self.thread.borrow_mut().take() {
            let _ = self.sender.send(Message::Stop);
            let _ = thread.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.shutdown();
    }
}
