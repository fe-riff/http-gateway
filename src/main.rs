#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate futures_cpupool;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate tokio_minihttp;
extern crate tokio_proto;
extern crate tokio_service;

use std::io;

use futures::Future;
use futures_cpupool::CpuPool;
use rand::Rng;
use tokio_minihttp::{Request, Response};
use tokio_proto::TcpServer;
use tokio_service::Service;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Serialize)]
struct Message {
    id: i32,
    body: String,
}

struct Server {
    thread_pool: CpuPool,
    reply_chan: Arc<Mutex<Receiver<i32>>>,
}

impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = Box<Future<Item = Response, Error = io::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let random_id = rand::thread_rng().gen_range(1, 5);
        let msg = self.thread_pool.spawn_fn(move || {
            Ok(Message {
                id: self.reply_chan.lock().unwrap().recv().unwrap(),
                body: String::from("hello"),
            })
        });

        Box::new(msg.map(|msg| {
            let json = serde_json::to_string(&msg).unwrap();
            let mut response = Response::new();

            response.header("Content-Type", "application/json");
            response.body(&json);
            response
        }))
    }
}

fn main() {
    // curl 127.0.0.1:8080 drives this successfully, but localhost:8080 doesn't!
    let addr = "127.0.0.1:8080".parse().unwrap();
    let thread_pool = CpuPool::new(10);

    let (sender, receiver) = channel();

    let receiver = Arc::new(Mutex::new(receiver));

    let fut = thread_pool.spawn_fn(move || {
        loop {
            sender.send(99);
        }
        Ok(())
    });

    TcpServer::new(tokio_minihttp::Http, addr).serve(move || {
        Ok(Server {
            thread_pool: thread_pool.clone(),
            reply_chan: Arc::clone(&receiver),
        })
    })
}
