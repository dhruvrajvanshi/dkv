use std::{ptr, time::SystemTime};

use libc::{exit, kevent, perror};

fn main() {
    let mut kq = KQueue::new();
    kq.register(
        1,
        libc::EVFILT_TIMER,
        libc::EV_ADD | libc::EV_ONESHOT,
        0,
        1000,
        std::ptr::null_mut(),
    );
    kq.register(
        2,
        libc::EVFILT_TIMER,
        libc::EV_ADD | libc::EV_ONESHOT,
        0,
        2000,
        ptr::null_mut(),
    );

    let start = SystemTime::now();
    let mut max_ticks = 10;
    while max_ticks > 0 {
        let event = kq.select_1();
        print!(
            "{}: ",
            SystemTime::now().duration_since(start).unwrap().as_secs()
        );
        let data: isize = match event.ident {
            1 => {
                println!("Timer 1 expired");
                1000
            }
            2 => {
                println!("Timer 2 expired");
                2000
            }
            _ => {
                panic!("Unexpected event");
            }
        };
        kq.register(
            event.ident,
            event.filter,
            event.flags,
            event.fflags,
            data,
            event.udata,
        );
        max_ticks -= 1;
    }
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
        &mut self,
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

    pub fn select_1(&mut self) -> libc::kevent {
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
