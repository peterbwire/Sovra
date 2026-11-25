//! Optional per-execution-domain fixed-window HTTP attempt admission; no per-identity quotas.
use super::HostError;
use std::time::{Duration, Instant};
/// Trusted fixed-window limits. Absence of this policy preserves existing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpAdmissionPolicy {
    /// Attempts per window, 0..1024. Zero denies all; failures still consume attempts.
    pub requests: usize,
    /// Window duration, 1..60000 milliseconds. Monotonic, no accrued idle credits.
    pub milliseconds: u64,
}
impl HttpAdmissionPolicy {
    pub(super) fn validate(self) -> Result<(), HostError> {
        if self.requests > 1024 || self.milliseconds == 0 || self.milliseconds > 60000 {
            return Err(HostError {
                code: "policy",
                message: "invalid HTTP admission limits".into(),
            });
        }
        Ok(())
    }
}
#[derive(Debug)]
pub(super) struct HttpAdmission {
    policy: HttpAdmissionPolicy,
    start: Instant,
    used: usize,
}
impl HttpAdmission {
    pub(super) fn new(policy: HttpAdmissionPolicy) -> Result<Self, HostError> {
        policy.validate()?;
        Ok(Self {
            policy,
            start: Instant::now(),
            used: 0,
        })
    }
    pub(super) fn admit(&mut self) -> bool {
        self.admit_at(Instant::now())
    }
    fn admit_at(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.start)
            >= Duration::from_millis(self.policy.milliseconds)
        {
            self.start = now;
            self.used = 0;
        }
        if self.used >= self.policy.requests {
            return false;
        }
        self.used += 1;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_window_boundary_resets_without_borrowing_or_idle_credits() {
        let mut limiter = HttpAdmission::new(HttpAdmissionPolicy {
            requests: 2,
            milliseconds: 100,
        })
        .unwrap();
        let t = limiter.start;
        assert!(limiter.admit_at(t));
        assert!(limiter.admit_at(t));
        assert!(!limiter.admit_at(t + Duration::from_millis(99)));
        assert!(limiter.admit_at(t + Duration::from_millis(100)));
        assert!(limiter.admit_at(t + Duration::from_millis(100)));
        assert!(!limiter.admit_at(t + Duration::from_millis(100)));
        assert!(limiter.admit_at(t + Duration::from_secs(100)));
        assert!(limiter.admit_at(t + Duration::from_secs(100)));
        assert!(!limiter.admit_at(t + Duration::from_secs(100)));
    }
    #[test]
    fn zero_admission_never_grants_and_clock_regression_does_not_reset() {
        let mut deny = HttpAdmission::new(HttpAdmissionPolicy {
            requests: 0,
            milliseconds: 1,
        })
        .unwrap();
        let t = deny.start;
        assert!(!deny.admit_at(t));
        assert!(!deny.admit_at(t + Duration::from_secs(1)));
        let mut one = HttpAdmission::new(HttpAdmissionPolicy {
            requests: 1,
            milliseconds: 100,
        })
        .unwrap();
        let t = one.start;
        assert!(one.admit_at(t));
        assert!(!one.admit_at(t - Duration::from_millis(1)));
        assert!(!one.admit_at(t));
    }
    #[test]
    fn policy_caps_refuse_before_counter_creation() {
        for policy in [
            HttpAdmissionPolicy {
                requests: 1025,
                milliseconds: 1,
            },
            HttpAdmissionPolicy {
                requests: 1,
                milliseconds: 0,
            },
            HttpAdmissionPolicy {
                requests: 1,
                milliseconds: 60001,
            },
        ] {
            assert_eq!(HttpAdmission::new(policy).unwrap_err().code, "policy");
        }
    }
}
