use std::{cell::RefCell, collections::HashMap, ptr, sync::Mutex};

use libc::{exit, kevent, perror};

fn main() {
    let event_loop = EventLoop::new();
    timeout(
        &event_loop,
        500,
        Box::new(|el| {
            println!("Once at 0.5 seconds");
            timeout(
                el,
                500,
                Box::new(|el| {
                    println!("Once at 1 second");
                    println!("Starting a new interval");
                    interval(
                        &el,
                        500,
                        Box::new(|_| {
                            println!("Every 0.5 seconds");
                        }),
                    );
                }),
            );
        }),
    );
    interval(
        &event_loop,
        1000,
        Box::new(|_| {
            println!("Every 1 second");
        }),
    );

    event_loop.run();
}

type Callback = Box<dyn Fn(&EventLoop) -> ()>;

struct EventLoop {
    kq: KQueue,
    next_id: RefCell<usize>,
    callbacks: Mutex<HashMap<usize, Callback>>,
    /**
     * We want to allow callbacks to register new callbacks,
     * but while callbacks are running, self.callbacks is locked.
     * So in case when during a register call, self.callbacks is locked,
     * we put the new registration here.
     * Every loop iteration, before processing events, we drain this list
     * into self.callbacks.
     */
    pending_registrations: Mutex<Vec<(usize, Callback)>>,
}
impl EventLoop {
    pub fn new() -> EventLoop {
        EventLoop {
            kq: KQueue::new(),
            next_id: RefCell::new(1),
            callbacks: Mutex::new(HashMap::new()),
            pending_registrations: Mutex::new(Vec::new()),
        }
    }
    pub fn register(
        &self,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: isize,
        udata: *mut libc::c_void,
        cb: Callback,
    ) -> usize {
        let ident = { *self.next_id.borrow() };
        {
            *self.next_id.borrow_mut() += 1;
        }
        self.kq.register(ident, filter, flags, fflags, data, udata);
        {
            match self.callbacks.try_lock() {
                Ok(mut callbacks) => {
                    callbacks.insert(ident, cb);
                }
                Err(_) => {
                    self.pending_registrations.lock().unwrap().push((ident, cb));
                }
            }
        }
        ident
    }

    pub fn run(&self) {
        loop {
            let event = self.kq.select_1();
            let ident = event.ident;
            for (ident, cb) in self.pending_registrations.lock().unwrap().drain(..) {
                match self.callbacks.try_lock() {
                    Ok(mut callbacks) => {
                        callbacks.insert(ident, cb);
                    }
                    Err(_) => {}
                }
            }
            match self.callbacks.try_lock() {
                Ok(callbacks) => {
                    if let Some(cb) = callbacks.get(&ident) {
                        cb(self);
                    } // else, event cancelled
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
}

fn timeout(event_loop: &EventLoop, time: isize, cb: Callback) -> usize {
    event_loop.register(
        libc::EVFILT_TIMER,
        libc::EV_ADD | libc::EV_ONESHOT,
        0,
        time,
        ptr::null_mut(),
        cb,
    )
}

fn interval(event_loop: &EventLoop, time: isize, cb: Callback) -> usize {
    event_loop.register(
        libc::EVFILT_TIMER,
        libc::EV_ADD,
        0,
        time,
        ptr::null_mut(),
        cb,
    )
}

struct KQueue {
    kq: i32,
}

impl KQueue {
    pub fn new() -> KQueue {
        let kq = unsafe { libc::kqueue() };
        assert!(kq >= 0, "Unable to create kqueue");
        KQueue { kq }
    }
    /// Escape hatch to call native kqueue APIs
    #[allow(dead_code)]
    pub fn raw(&mut self) -> i32 {
        self.kq
    }

    pub fn register(
        &self,
        ident: usize,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: isize,
        udata: *mut libc::c_void,
    ) {
        let event = libc::kevent {
            ident,
            filter,
            flags,
            fflags,
            data,
            udata,
        };
        unsafe {
            assert!(
                kevent(
                    self.kq,
                    &event as *const kevent,
                    1,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                ) >= 0,
                "Unable to add events to kqueue",
            )
        }
    }

    pub fn select_1(&self) -> libc::kevent {
        let mut event = libc::kevent {
            ident: 1,
            filter: libc::EVFILT_TIMER,
            flags: libc::EV_ADD | libc::EV_ONESHOT,
            fflags: 0,
            data: 5000,
            udata: 0 as *mut _,
        };
        unsafe {
            let num_events = kevent(
                self.kq,
                std::ptr::null(),
                0,
                (&mut event) as *mut kevent,
                1,
                std::ptr::null(),
            );
            if num_events < 0 {
                perror(std::ptr::null());
                exit(1);
            }
        }
        event
    }
}

impl Drop for KQueue {
    fn drop(&mut self) {
        unsafe { libc::close(self.kq) };
    }
}
