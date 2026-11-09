use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Shared admission state for cooperating writers and cancellation controllers.
///
/// A write is accepted when `try_accept` observes an open gate under the same
/// mutex used by `cancel`. Cancellation cannot revoke writes already accepted,
/// and does not interrupt work already running. External writers can bypass it.
#[derive(Clone)]
pub struct AdmissionGate {
    state: Arc<Mutex<State>>,
    cancellation: watch::Sender<bool>,
    released: watch::Sender<u64>,
}

#[derive(Default)]
struct State {
    open: bool,
    cancelled: bool,
    active: u64,
}

/// Proof that a write was accepted before the gate closed or was cancelled.
/// Cooperating writers must keep this value until their write has finished.
/// Closing admission waits for outstanding values; forgetting one can stall it.
#[must_use]
pub struct AcceptedWrite {
    state: Arc<Mutex<State>>,
    released: watch::Sender<u64>,
}

/// A process-local bound on caller-declared units held by admitted operations.
///
/// This is not a measurement of SQLite WAL, spool, or remote storage. Every
/// producer must share this instance and provide a conservative cost in the
/// same units. Reservations are volatile and released when their permit drops.
#[derive(Clone)]
pub struct CapacityGate {
    admission: AdmissionGate,
    capacity: Arc<Mutex<CapacityState>>,
}

struct CapacityState {
    limit: u64,
    used: u64,
}

/// Holds both admission and the declared capacity reservation.
#[must_use]
pub struct CapacityPermit {
    admission: Option<AcceptedWrite>,
    capacity: Arc<Mutex<CapacityState>>,
    units: u64,
}

impl Drop for CapacityPermit {
    fn drop(&mut self) {
        if let Ok(mut capacity) = self.capacity.lock() {
            capacity.used -= self.units;
        }
        // Release the admission permit only after releasing its reservation.
        drop(self.admission.take());
    }
}

impl CapacityGate {
    /// Create a bounded reference controller; zero capacity is invalid.
    pub fn new(admission: AdmissionGate, limit: u64) -> io::Result<Self> {
        if limit == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "capacity must be positive",
            ));
        }
        Ok(Self {
            admission,
            capacity: Arc::new(Mutex::new(CapacityState { limit, used: 0 })),
        })
    }

    /// Reserve units and accept a write without waiting; the permit must be
    /// retained until both the operation and its temporary reservation end.
    pub fn try_reserve(&self, units: u64) -> io::Result<CapacityPermit> {
        if units == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "reservation must be positive",
            ));
        }
        let mut capacity = self.capacity.lock().map_err(|_| poisoned())?;
        let used = capacity
            .used
            .checked_add(units)
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "capacity exhausted"))?;
        if used > capacity.limit {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "capacity exhausted",
            ));
        }
        let admission = self.admission.try_accept()?;
        capacity.used = used;
        Ok(CapacityPermit {
            admission: Some(admission),
            capacity: Arc::clone(&self.capacity),
            units,
        })
    }
}

impl Drop for AcceptedWrite {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.active -= 1;
            self.released.send_modify(|generation| {
                *generation = generation.wrapping_add(1);
            });
        }
    }
}

impl AdmissionGate {
    /// Construct a gate that initially denies writes.
    pub fn new() -> Self {
        let (cancellation, _) = watch::channel(false);
        let (released, _) = watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(State::default())),
            cancellation,
            released,
        }
    }

    /// Open the gate unless it has been permanently cancelled.
    pub(crate) fn open(&self) -> io::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if state.cancelled {
            return Err(io::Error::other("write admission was cancelled"));
        }
        state.open = true;
        Ok(())
    }

    /// Deny subsequent writes. Previously accepted writes are not interrupted.
    pub(crate) fn close(&self) -> io::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        state.open = false;
        Ok(())
    }

    /// Permanently deny subsequent writes, atomically with acceptance.
    pub fn cancel(&self) -> io::Result<()> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        state.cancelled = true;
        state.open = false;
        self.cancellation.send_replace(true);
        Ok(())
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.cancellation.subscribe()
    }

    pub(crate) fn is_cancelled(&self) -> io::Result<bool> {
        let state = self.state.lock().map_err(|_| poisoned())?;
        Ok(state.cancelled)
    }

    /// Wait until all cooperating writes accepted before closure have finished.
    pub(crate) async fn drain(&self) -> io::Result<()> {
        let mut released = self.released.subscribe();
        loop {
            let active = self.state.lock().map_err(|_| poisoned())?.active;
            if active == 0 {
                return Ok(());
            }
            released
                .changed()
                .await
                .map_err(|_| io::Error::other("write admission notification closed"))?;
        }
    }

    /// Accept a write at this call's linearization point, or fail closed.
    ///
    /// This only synchronizes admission and draining. It does not reserve WAL,
    /// spool, upload, or other capacity; the adapter and every writer must
    /// enforce the configured limits separately before performing a write.
    pub fn try_accept(&self) -> io::Result<AcceptedWrite> {
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if state.cancelled || !state.open {
            return Err(io::Error::other("write admission is closed"));
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or_else(|| io::Error::other("too many accepted writes"))?;
        Ok(AcceptedWrite {
            state: Arc::clone(&self.state),
            released: self.released.clone(),
        })
    }
}

impl Default for AdmissionGate {
    fn default() -> Self {
        Self::new()
    }
}

fn poisoned() -> io::Error {
    io::Error::other("write admission state is poisoned; writes are denied")
}

#[cfg(test)]
mod tests {
    use super::{AdmissionGate, CapacityGate};
    use std::sync::{Arc, Barrier};

    #[test]
    fn cancellation_is_monotonic_and_fails_closed() {
        let gate = AdmissionGate::new();
        assert!(gate.try_accept().is_err());
        gate.open().expect("open");
        let _accepted = gate.try_accept().expect("accept before cancellation");
        gate.cancel().expect("cancel");
        assert!(gate.open().is_err());
        assert!(gate.try_accept().is_err());
        gate.close().expect("close after cancellation");
        assert!(gate.open().is_err());
    }

    #[test]
    fn writers_cannot_be_accepted_after_cancel_returns() {
        let gate = AdmissionGate::new();
        gate.open().expect("open");
        let barrier = Arc::new(Barrier::new(2));
        let worker_gate = gate.clone();
        let worker_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            for _ in 0..1000 {
                assert!(worker_gate.try_accept().is_err());
            }
        });
        gate.cancel().expect("cancel");
        barrier.wait();
        worker.join().expect("worker");
    }

    #[test]
    fn acceptance_racing_cancellation_obeys_linearization() {
        const ROUNDS: usize = 200;
        const WRITERS: usize = 8;

        for _ in 0..ROUNDS {
            let gate = AdmissionGate::new();
            gate.open().expect("open");
            let start = Arc::new(Barrier::new(WRITERS + 1));
            std::thread::scope(|scope| {
                let mut writers = Vec::new();
                for _ in 0..WRITERS {
                    let gate = gate.clone();
                    let start = Arc::clone(&start);
                    writers.push(scope.spawn(move || {
                        start.wait();
                        // An acceptance here may linearize on either side of cancel.
                        let _racing_result = gate.try_accept();
                    }));
                }
                start.wait();
                gate.cancel().expect("cancel");
                for writer in writers {
                    writer.join().expect("racing writer");
                }
            });
            for _ in 0..WRITERS {
                assert!(gate.try_accept().is_err());
            }
            assert!(gate.open().is_err());
        }
    }

    #[tokio::test]
    async fn poisoned_state_fails_closed_even_with_outstanding_writer() {
        let gate = AdmissionGate::new();
        gate.open().expect("open");
        let permit = gate.try_accept().expect("accept writer");
        let state = Arc::clone(&gate.state);
        let worker = std::thread::spawn(move || {
            let _lock = state.lock().expect("state lock");
            panic!("poison admission state for regression test");
        });
        assert!(worker.join().is_err());
        drop(permit);
        assert!(gate.try_accept().is_err());
        assert!(gate.cancel().is_err());
        assert!(gate.drain().await.is_err());
    }

    #[test]
    fn capacity_reservations_are_bounded_released_and_cancelled() {
        let gate = AdmissionGate::new();
        assert!(CapacityGate::new(gate.clone(), 0).is_err());
        let capacity = CapacityGate::new(gate.clone(), 10).expect("capacity");
        assert!(capacity.try_reserve(1).is_err());
        gate.open().expect("open");
        assert!(capacity.try_reserve(0).is_err());
        let first = capacity.try_reserve(7).expect("reserve seven");
        assert_eq!(
            capacity.try_reserve(4).err().expect("full").kind(),
            std::io::ErrorKind::WouldBlock
        );
        let second = capacity.try_reserve(3).expect("reserve remaining");
        assert!(capacity.try_reserve(u64::MAX).is_err());
        drop(first);
        let third = capacity
            .try_reserve(7)
            .expect("released capacity available");
        gate.cancel().expect("cancel");
        drop(second);
        drop(third);
        assert!(capacity.try_reserve(1).is_err());
    }

    #[test]
    fn concurrent_reservations_do_not_exceed_capacity() {
        const WRITERS: usize = 16;
        let gate = AdmissionGate::new();
        gate.open().expect("open");
        let capacity = CapacityGate::new(gate, 2).expect("capacity");
        let start = Arc::new(Barrier::new(WRITERS + 1));
        let finish = Arc::new(Barrier::new(WRITERS + 1));
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..WRITERS {
                let capacity = capacity.clone();
                let start = Arc::clone(&start);
                let finish = Arc::clone(&finish);
                handles.push(scope.spawn(move || {
                    start.wait();
                    let permit = capacity.try_reserve(1).ok();
                    finish.wait();
                    permit.is_some()
                }));
            }
            start.wait();
            finish.wait();
            let mut accepted = 0;
            for handle in handles {
                if handle.join().expect("writer") {
                    accepted += 1;
                }
            }
            assert_eq!(accepted, 2);
        });
        assert!(capacity.try_reserve(2).is_ok());
    }
}
