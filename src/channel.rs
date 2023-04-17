use core::cell::RefCell;

use alloc::{
    collections::VecDeque,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use critical_section::Mutex;

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, s: T) {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            q.push_front(s);
        });
    }

    pub fn send_all<I: IntoIterator<Item = T>>(&mut self, contents: I) {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            for i in contents.into_iter() {
                q.push_front(i);
            }
        })
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            q.pop_back()
        })
    }

    pub fn recv_all(&mut self) -> Recv<T> {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            let mut v = Vec::new();
            while let Some(i) = q.pop_back() {
                v.push(i);
            }
            Recv::new(v)
        })
    }
}

#[derive(Debug)]
struct Inner<T> {
    queue: Mutex<RefCell<VecDeque<T>>>,
}

pub struct Recv<T> {
    contents: Vec<T>,
}

impl<T> Recv<T> {
    pub fn new(contents: Vec<T>) -> Recv<T> {
        Recv { contents }
    }
}

impl<T> Iterator for Recv<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.contents.pop()
    }
}

impl ToString for Recv<char> {
    fn to_string(&self) -> String {
        self.contents.iter().collect()
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: Mutex::new(RefCell::new(VecDeque::new())),
    };
    let inner = Arc::new(inner);
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}
