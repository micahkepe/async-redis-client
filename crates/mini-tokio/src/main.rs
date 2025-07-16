//! A minimal version of Tokio to understand the runtime.
use futures::task::{self, ArcWake};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, mpsc};
use std::task::{Context, Poll};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

struct MiniTokio {
    scheduled: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
}

struct TaskFuture {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    poll: Poll<()>,
}

impl TaskFuture {
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            future: Box::pin(future),
            poll: Poll::Pending,
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) {
        // Spurious wake-ups are allowed, even after a future has
        // returned `Ready`. However, polling a future which has
        // already returned `Ready` is *not* allowed. For this
        // reason we need to check that the future is still pending
        // before we call it. Failure to do so can lead to a panic.
        if self.poll.is_pending() {
            self.poll = self.future.as_mut().poll(cx);
        }
    }
}

struct Task {
    task_future: Mutex<TaskFuture>,
    executor: mpsc::Sender<Arc<Task>>,
}

impl Task {
    fn spawn<F>(future: F, sender: &mpsc::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            task_future: Mutex::new(TaskFuture::new(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }

    fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        let mut task_future = self.task_future.try_lock().unwrap();
        task_future.poll(&mut cx);
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.executor.send(arc_self.clone());
    }
}

impl MiniTokio {
    fn new() -> MiniTokio {
        let (sender, scheduled) = mpsc::channel();
        MiniTokio { sender, scheduled }
    }

    fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender)
    }

    fn run(&mut self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}

// struct Delay {
//     when: Instant,
// }
//
// impl Future for Delay {
//     type Output = &'static str;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         if Instant::now() >= self.when {
//             println!("Hello world");
//             Poll::Ready("done")
//         } else {
//             // Get handle to the waker for the current task
//             let waker = cx.waker().clone();
//             let when = self.when;
//
//             // Spawn timer thread
//             thread::spawn(move || {
//                 let now = Instant::now();
//                 if now < when {
//                     thread::sleep(when - now);
//                 }
//
//                 waker.wake();
//             });
//
//             Poll::Pending
//         }
//     }
// }

async fn delay(dur: Duration) {
    let when = Instant::now() + dur;
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    thread::spawn(move || {
        let now = Instant::now();

        if now < when {
            thread::sleep(when - now);
        }

        notify_clone.notify_one();
    });

    notify.notified().await;
}

fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        delay(Duration::from_millis(10)).await;
        println!("Hello world");
    });

    mini_tokio.run();
}
