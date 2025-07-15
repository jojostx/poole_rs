use core::fmt;
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum Message {
    Shutdown(usize),
    Job(Job),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Shutdown(id) => f.debug_tuple("Shutdown").field(id).finish(),
            Message::Job(_) => f.write_str("Job(<FnOnce>)"),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

struct Worker {
    id: usize,
    join_handle: Option<JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(count: usize) -> ThreadPool {
        let mut workers = vec![];
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        // spawn the threads in a loop, and create a worker
        for id in 1..=count {
            let receiver = receiver.clone();
            let join_handle = thread::spawn(move || loop {
                let message = receiver.lock().unwrap();
                let message: Message = message.recv().unwrap();

                match message {
                    // check if message is a job or a shutdown message
                    Message::Shutdown(id_) => {
                        if id_ == id {
                            break;
                        }
                    }
                    Message::Job(job) => {
                        job();
                    }
                }
            });
            workers.push(Worker {
                id,
                join_handle: Some(join_handle),
            });
        }

        //  add the workers and the sender to the threadpool
        ThreadPool { workers, sender }
    }

    pub fn execute(&self, job: Message) {
        self.sender.send(job).unwrap();
    }

    pub fn shutdown(&mut self, id: Option<usize>) {
        match id {
            Some(id) => self.sender.send(Message::Shutdown(id)).unwrap(),
            None => {
                for worker in &self.workers {
                    self.sender.send(Message::Shutdown(worker.id)).unwrap();
                }
            }
        };

        for worker in &mut self.workers {
            if let Some(handle) = worker.join_handle.take() {
                handle.join().unwrap();
            }
        }
    }
}
