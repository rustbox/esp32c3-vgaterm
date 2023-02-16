use core::cell::RefCell;

use alloc::{collections::VecDeque, rc::Rc};
use critical_section::Mutex;

pub struct Sender<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, s: T) {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            q.push_front(s);
        });
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender {
            inner: Rc::clone(&self.inner),
        }
    }
}

pub struct Receiver<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        critical_section::with(|cs| {
            let mut q = self.inner.queue.borrow(cs).borrow_mut();
            q.pop_back()
        })
    }
}

struct Inner<T> {
    queue: Mutex<RefCell<VecDeque<T>>>,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Inner {
        queue: Mutex::new(RefCell::new(VecDeque::new())),
    };
    let inner = Rc::new(inner);
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver {
            inner,
        },
    )
}
