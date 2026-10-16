use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn record_request() {
    REQUESTED.store(true, Ordering::Release);
}

pub fn take_request() -> bool {
    REQUESTED.swap(false, Ordering::AcqRel)
}

#[cfg(test)]
mod tests {
    use super::{record_request, take_request};

    #[test]
    fn native_quit_request_is_delivered_once_to_root_coordinator() {
        assert!(!take_request());
        record_request();
        assert!(take_request());
        assert!(!take_request());
    }
}
