//! Experimental logical accounting; this is not a filesystem quota.
use std::io;
use std::sync::{Arc, Mutex};

use crate::{AcceptedWrite, AdmissionGate};

/// Single-resource, process-local reference accounting. Callers must provide
/// conservative values, cover all producers, and reconcile after restart.
#[derive(Clone)]
pub struct ConservativeBudget {
    admission: AdmissionGate,
    state: Arc<Mutex<State>>,
}

struct State {
    limit: u64,
    persisted: Option<u64>,
    reserved: u64,
    active: u64,
}

/// Dropping without settlement invalidates occupancy until reconciliation.
#[must_use]
pub struct BudgetReservation {
    state: Arc<Mutex<State>>,
    admission: Option<AcceptedWrite>,
    reserved: u64,
    settled: bool,
}

impl ConservativeBudget {
    /// Starts closed to reservations until occupancy has been reconciled.
    pub fn new(admission: AdmissionGate, limit: u64) -> io::Result<Self> {
        if limit == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "zero limit"));
        }
        Ok(Self {
            admission,
            state: Arc::new(Mutex::new(State {
                limit,
                persisted: None,
                reserved: 0,
                active: 0,
            })),
        })
    }

    /// Invalidate occupancy, including while reservations are active.
    /// Precomputed values are rejected; use `reconcile_with` to measure while
    /// reservations through this budget are excluded.
    pub fn reconcile(&self, occupancy: Option<u64>) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("budget poisoned"))?;
        if occupancy.is_none() {
            state.persisted = None;
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "precomputed occupancy rejected; use reconcile_with",
        ))
    }

    /// Measure and install occupancy while excluding reservations through this
    /// exact budget. The callback must not reenter this budget and must also
    /// exclude external writers, capture, and other independently created
    /// budgets. A measurement error or unknown result leaves occupancy unknown.
    /// If reservations are active, the callback is not called and the existing
    /// occupancy is unchanged; callers that detect an independent loss of
    /// confidence must explicitly invalidate it with `reconcile(None)`. This
    /// synchronous callback holds a mutex and can block all budget callers;
    /// it must be bounded and must not reenter this budget.
    pub fn reconcile_with<F>(&self, measure: F) -> io::Result<()>
    where
        F: FnOnce() -> io::Result<Option<u64>>,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("budget poisoned"))?;
        if state.active != 0 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "reservations active",
            ));
        }
        state.persisted = None;
        let occupancy = measure()?;
        state.persisted = occupancy;
        if occupancy.is_none() {
            return Err(io::Error::other("occupancy unknown"));
        }
        Ok(())
    }

    /// Run a cooperative occupancy-changing operation exclusively against
    /// reconciliation and reservations using this same budget. Occupancy is
    /// invalidated before the operation, including if it fails. No active
    /// reservations may exist. The callback must not reenter this budget.
    /// This does not coordinate other instances or uncooperative operations.
    pub fn with_occupancy_mutation<T, F>(&self, operation: F) -> io::Result<T>
    where
        F: FnOnce() -> io::Result<T>,
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("budget poisoned"))?;
        if state.active != 0 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "reservations active",
            ));
        }
        state.persisted = None;
        operation()
    }

    /// Reject unknown costs or occupancy; do not wait on capacity.
    /// Acquiring the mutex may block, especially during `reconcile_with`;
    /// this API does not guarantee bounded latency or prompt cancellation.
    pub fn try_reserve(&self, cost: Option<u64>) -> io::Result<BudgetReservation> {
        let cost = cost
            .filter(|value| *value > 0)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown or zero cost"))?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("budget poisoned"))?;
        let persisted = state
            .persisted
            .ok_or_else(|| io::Error::other("occupancy unknown"))?;
        let total = persisted
            .checked_add(state.reserved)
            .and_then(|n| n.checked_add(cost));
        if total.is_none_or(|n| n > state.limit) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "budget exhausted",
            ));
        }
        let admission = self.admission.try_accept()?;
        state.reserved += cost;
        state.active += 1;
        Ok(BudgetReservation {
            state: Arc::clone(&self.state),
            admission: Some(admission),
            reserved: cost,
            settled: false,
        })
    }
}

impl BudgetReservation {
    /// Settle using a proven conservative increase, including on rollback.
    /// Unknown or underestimated growth invalidates occupancy until reconciliation.
    pub fn settle(mut self, increase: Option<u64>) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("budget poisoned"))?;
        let updated = match (state.persisted, increase) {
            (Some(value), Some(growth)) if growth <= self.reserved => value.checked_add(growth),
            _ => None,
        };
        state.persisted = updated;
        state.reserved -= self.reserved;
        state.active -= 1;
        self.settled = true;
        if updated.is_none() {
            return Err(io::Error::other("occupancy requires reconciliation"));
        }
        Ok(())
    }
}

impl Drop for BudgetReservation {
    fn drop(&mut self) {
        if !self.settled
            && let Ok(mut state) = self.state.lock()
        {
            state.persisted = None;
            state.reserved -= self.reserved;
            state.active -= 1;
        }
        drop(self.admission.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooperative_mutation_waits_for_measurement_and_invalidates_it() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        let (measuring_tx, measuring_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (mutation_tx, mutation_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let measuring = budget.clone();
            scope.spawn(move || {
                measuring
                    .reconcile_with(|| {
                        measuring_tx.send(()).expect("started");
                        release_rx.recv().expect("release");
                        Ok(Some(0))
                    })
                    .expect("measure");
            });
            measuring_rx.recv().expect("measurement started");
            let mutating = budget.clone();
            scope.spawn(move || {
                mutating
                    .with_occupancy_mutation(|| {
                        mutation_tx.send(()).expect("mutation entered");
                        Ok(())
                    })
                    .expect("mutation");
            });
            assert!(
                mutation_rx
                    .recv_timeout(std::time::Duration::from_millis(100))
                    .is_err()
            );
            release_tx.send(()).expect("release");
            mutation_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("mutation completed");
        });
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "mutation invalidated measurement"
        );
    }

    #[test]
    fn active_reservation_prevents_measurement_without_invalidating() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        budget.reconcile_with(|| Ok(Some(2))).expect("baseline");
        let reservation = budget.try_reserve(Some(2)).expect("reserve");
        let mut called = false;
        let error = budget
            .reconcile_with(|| {
                called = true;
                Ok(Some(0))
            })
            .expect_err("active reservation prevents measurement");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(!called);
        reservation.settle(Some(2)).expect("settle known growth");
        assert!(budget.try_reserve(Some(7)).is_err());
        budget.reconcile(None).expect("explicit invalidation");
        assert!(budget.try_reserve(Some(1)).is_err());
        budget.reconcile_with(|| Ok(Some(4))).expect("recovery");
        budget
            .try_reserve(Some(6))
            .expect("remaining capacity")
            .settle(Some(0))
            .expect("settle");
    }

    #[test]
    fn rejects_unknown_and_tracks_persisted_and_reserved_separately() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        assert!(budget.try_reserve(Some(1)).is_err());
        budget
            .reconcile_with(|| Ok(Some(4)))
            .expect("initial occupancy");
        assert!(budget.try_reserve(None).is_err());
        let first = budget.try_reserve(Some(4)).expect("reserve");
        assert!(budget.reconcile_with(|| Ok(Some(0))).is_err());
        assert!(budget.try_reserve(Some(3)).is_err());
        first.settle(Some(3)).expect("settle");
        assert!(budget.try_reserve(Some(4)).is_err());
        budget
            .try_reserve(Some(3))
            .expect("remaining")
            .settle(Some(0))
            .expect("rollback accounting");
        budget
            .reconcile_with(|| Ok(Some(2)))
            .expect("reclaim after measurement");
        assert!(budget.try_reserve(Some(8)).is_ok());
    }

    #[test]
    fn abandoned_or_underestimated_reservation_requires_reconciliation() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate.clone(), 10).expect("budget");
        budget.reconcile_with(|| Ok(Some(0))).expect("occupancy");
        drop(budget.try_reserve(Some(3)).expect("reserve"));
        assert!(budget.try_reserve(Some(1)).is_err());
        budget.reconcile_with(|| Ok(Some(1))).expect("reconcile");
        assert!(
            budget
                .try_reserve(Some(2))
                .expect("reserve")
                .settle(Some(3))
                .is_err()
        );
        assert!(budget.try_reserve(Some(1)).is_err());
        budget.reconcile(None).expect("unknown occupancy");
        assert!(budget.try_reserve(Some(1)).is_err());
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("reconcile again");
        gate.cancel().expect("cancel");
        assert!(budget.try_reserve(Some(1)).is_err());
    }

    #[test]
    fn concurrent_reservations_cancel_and_reconcile_without_overcommit() {
        const WRITERS: usize = 12;
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate.clone(), 5).expect("budget");
        budget.reconcile_with(|| Ok(Some(2))).expect("baseline");
        let start = Arc::new(std::sync::Barrier::new(WRITERS + 1));
        let finish = Arc::new(std::sync::Barrier::new(WRITERS + 1));
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            for _ in 0..WRITERS {
                let budget = budget.clone();
                let start = Arc::clone(&start);
                let finish = Arc::clone(&finish);
                let sender = sender.clone();
                scope.spawn(move || {
                    start.wait();
                    let reservation = budget.try_reserve(Some(1));
                    sender.send(reservation.is_ok()).expect("report result");
                    finish.wait();
                    if let Ok(reservation) = reservation {
                        reservation.settle(Some(1)).expect("settle accepted write");
                    }
                });
            }
            start.wait();
            let mut accepted = 0;
            for _ in 0..WRITERS {
                if receiver.recv().expect("writer result") {
                    accepted += 1;
                }
            }
            assert_eq!(accepted, 3);
            assert_eq!(
                budget
                    .reconcile_with(|| Ok(Some(0)))
                    .expect_err("active reservations")
                    .kind(),
                io::ErrorKind::WouldBlock
            );
            gate.cancel().expect("cancel admission");
            assert!(budget.try_reserve(Some(1)).is_err());
            finish.wait();
        });
        {
            let state = budget.state.lock().expect("state");
            assert_eq!(state.persisted, Some(5));
            assert_eq!(state.reserved, 0);
            assert_eq!(state.active, 0);
        }
        budget
            .reconcile_with(|| Ok(Some(2)))
            .expect("reconcile after drain");
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "cancel remains permanent"
        );
    }

    #[test]
    fn unknown_occupancy_invalidates_even_with_active_reservations() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
        let reservation = budget.try_reserve(Some(2)).expect("active reservation");
        budget.reconcile(None).expect("invalidate while active");
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "unknown occupancy must deny admission"
        );
        assert!(
            reservation.settle(Some(1)).is_err(),
            "settling cannot restore unknown occupancy"
        );
        assert!(budget.try_reserve(Some(1)).is_err());
        budget
            .reconcile_with(|| Ok(Some(1)))
            .expect("reconcile after drain");
        budget
            .try_reserve(Some(1))
            .expect("known occupancy")
            .settle(Some(0))
            .expect("settle");
    }

    #[test]
    fn precomputed_stale_measurement_is_rejected() {
        let gate = AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("initial occupancy");
        // Simulates a measurement taken before a complete intervening write.
        let stale_measurement = 0;
        budget
            .try_reserve(Some(8))
            .expect("writer")
            .settle(Some(8))
            .expect("persisted growth");
        assert_eq!(
            budget
                .reconcile(Some(stale_measurement))
                .expect_err("precomputed value rejected")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(budget.try_reserve(Some(3)).is_err());
    }

    #[test]
    fn synchronized_measurement_excludes_reservations_and_fails_closed() {
        let gate = AdmissionGate::new();
        gate.open().expect("open");
        let budget = ConservativeBudget::new(gate, 10).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("initial measurement");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (attempt_tx, attempt_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let measuring = budget.clone();
            let worker = scope.spawn(move || {
                measuring
                    .reconcile_with(|| {
                        entered_tx.send(()).expect("signal measurement");
                        release_rx.recv().expect("release measurement");
                        Ok(Some(9))
                    })
                    .expect("measurement succeeds");
            });
            entered_rx.recv().expect("measurement started");
            let writing = budget.clone();
            let writer = scope.spawn(move || {
                let result = writing.try_reserve(Some(2));
                attempt_tx.send(result.is_err()).expect("report result");
            });
            assert!(
                attempt_rx
                    .recv_timeout(std::time::Duration::from_millis(100))
                    .is_err()
            );
            release_tx.send(()).expect("release");
            worker.join().expect("measurement thread");
            assert!(
                attempt_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .expect("writer result")
            );
            writer.join().expect("writer thread");
        });
        assert!(
            budget
                .reconcile_with(|| Err(io::Error::other("measurement failed")))
                .is_err()
        );
        assert!(budget.try_reserve(Some(1)).is_err());
        assert!(budget.reconcile_with(|| Ok(None)).is_err());
        assert!(budget.try_reserve(Some(1)).is_err());
        budget.reconcile_with(|| Ok(Some(9))).expect("recovery");
        budget
            .try_reserve(Some(1))
            .expect("remaining capacity")
            .settle(Some(0))
            .expect("settle");
    }

    #[test]
    fn cancellation_does_not_interrupt_wait_for_measurement_mutex() {
        let gate = AdmissionGate::new();
        gate.open().expect("open");
        let budget = ConservativeBudget::new(gate.clone(), 10).expect("budget");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let measuring = budget.clone();
            scope.spawn(move || {
                measuring
                    .reconcile_with(|| {
                        entered_tx.send(()).expect("signal entry");
                        release_rx.recv().expect("release");
                        Ok(Some(0))
                    })
                    .expect("measurement");
            });
            entered_rx.recv().expect("measurement entered");
            gate.cancel().expect("cancel gate");
            let writing = budget.clone();
            scope.spawn(move || {
                result_tx
                    .send(writing.try_reserve(Some(1)).is_err())
                    .expect("report result");
            });
            assert!(
                result_rx
                    .recv_timeout(std::time::Duration::from_millis(100))
                    .is_err()
            );
            release_tx.send(()).expect("release measurement");
            assert!(
                result_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .expect("result")
            );
        });
    }
}
