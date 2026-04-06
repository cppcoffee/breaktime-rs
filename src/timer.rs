use std::time::{Duration, Instant};

use winit::event_loop::ControlFlow;

pub const MIN_BREAK_MINUTES: u32 = 1;
pub const MAX_BREAK_MINUTES: u32 = 20;
pub const DEFAULT_BREAK_MINUTES: u32 = 20;

const BUBBLE_THRESHOLD: Duration = Duration::from_secs(10);
const BUBBLE_REFRESH_INTERVAL: Duration = Duration::from_millis(200);
const MIN_TRAY_ICON_WAKE_INTERVAL: Duration = Duration::from_secs(1);
const SNOOZE_DURATION: Duration = Duration::from_secs(60);
const TRAY_ICON_TOTAL_STEPS: u32 = 360;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    Idle,
    Counting,
    Alert,
}

#[derive(Debug, Clone, Copy)]
enum TimerState {
    Idle,
    Counting {
        deadline: Instant,
        total_duration: Duration,
    },
    Alert,
}

#[derive(Debug, Clone, Copy)]
pub struct BreakTimer {
    duration: Duration,
    state: TimerState,
}

impl BreakTimer {
    pub fn new(minutes: u32) -> Self {
        Self {
            duration: duration_from_minutes(minutes),
            state: TimerState::Idle,
        }
    }

    pub fn start(&mut self, now: Instant) {
        self.start_for_duration(self.duration, now);
    }

    pub fn start_snooze(&mut self, now: Instant) {
        self.start_for_duration(SNOOZE_DURATION, now);
    }

    fn start_for_duration(&mut self, duration: Duration, now: Instant) {
        self.state = TimerState::Counting {
            deadline: now + duration,
            total_duration: duration,
        };
    }

    pub fn stop(&mut self) {
        self.state = TimerState::Idle;
    }

    pub fn set_duration_minutes(&mut self, minutes: u32, now: Instant) {
        self.duration = duration_from_minutes(minutes);

        if matches!(self.state, TimerState::Counting { .. }) {
            self.start(now);
        }
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        match self.state {
            TimerState::Counting { deadline, .. } if now >= deadline => {
                self.state = TimerState::Alert;
                true
            }
            _ => false,
        }
    }

    pub fn mode(&self) -> TimerMode {
        match self.state {
            TimerState::Idle => TimerMode::Idle,
            TimerState::Counting { .. } => TimerMode::Counting,
            TimerState::Alert => TimerMode::Alert,
        }
    }

    pub fn bubble_seconds(&self, now: Instant) -> Option<u32> {
        match self.state {
            TimerState::Counting { deadline, .. } => {
                let remaining = deadline.saturating_duration_since(now);
                if remaining.is_zero() || remaining > BUBBLE_THRESHOLD {
                    None
                } else {
                    Some(ceil_seconds(remaining))
                }
            }
            _ => None,
        }
    }

    pub fn tray_icon_step(&self, now: Instant) -> Option<u32> {
        self.remaining(now)
            .map(|(remaining, total_duration)| remaining_tray_icon_steps(remaining, total_duration))
    }

    pub fn tray_icon_progress(&self, now: Instant) -> Option<f32> {
        self.remaining(now).map(|(remaining, total_duration)| {
            remaining.as_secs_f32() / total_duration.as_secs_f32()
        })
    }

    pub fn next_control_flow(&self, now: Instant) -> ControlFlow {
        match self.next_wake(now) {
            Some(next_wake) => ControlFlow::WaitUntil(next_wake),
            None => ControlFlow::Wait,
        }
    }

    fn remaining(&self, now: Instant) -> Option<(Duration, Duration)> {
        match self.state {
            TimerState::Counting {
                deadline,
                total_duration,
            } => Some((deadline.saturating_duration_since(now), total_duration)),
            TimerState::Idle | TimerState::Alert => None,
        }
    }

    fn next_wake(&self, now: Instant) -> Option<Instant> {
        match self.state {
            TimerState::Counting {
                deadline,
                total_duration,
            } => {
                let remaining = deadline.saturating_duration_since(now);
                if remaining.is_zero() {
                    return Some(now);
                }

                let tray_icon_wake = next_tray_icon_wake(now, deadline, remaining, total_duration);
                let bubble_wake = if remaining > BUBBLE_THRESHOLD {
                    deadline - BUBBLE_THRESHOLD
                } else {
                    (now + BUBBLE_REFRESH_INTERVAL).min(deadline)
                };

                Some(tray_icon_wake.min(bubble_wake))
            }
            TimerState::Idle | TimerState::Alert => None,
        }
    }
}

pub fn minutes_text(minutes: u32) -> String {
    format!("{minutes} min")
}

pub fn countdown_text(seconds: u32) -> String {
    format!("{seconds:02}")
}

fn duration_from_minutes(minutes: u32) -> Duration {
    Duration::from_secs(minutes as u64 * 60)
}

fn ceil_seconds(duration: Duration) -> u32 {
    duration.as_millis().div_ceil(1000) as u32
}

fn next_tray_icon_wake(
    now: Instant,
    deadline: Instant,
    remaining: Duration,
    total_duration: Duration,
) -> Instant {
    let remaining_steps = remaining_tray_icon_steps(remaining, total_duration);
    let exact_next_wake = if remaining_steps <= 1 {
        deadline
    } else {
        let nanos = total_duration
            .as_nanos()
            .saturating_mul(u128::from(remaining_steps - 1))
            / u128::from(TRAY_ICON_TOTAL_STEPS);
        let next_boundary_from_deadline =
            Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64);
        deadline - next_boundary_from_deadline
    };

    // Short durations can cross multiple icon steps per second. Clamp tray icon
    // wakeups to 1 Hz so the event loop does not spin on sub-second updates.
    exact_next_wake.max((now + MIN_TRAY_ICON_WAKE_INTERVAL).min(deadline))
}

fn remaining_tray_icon_steps(remaining: Duration, total_duration: Duration) -> u32 {
    if remaining.is_zero() {
        0
    } else {
        let value = remaining.as_nanos() * u128::from(TRAY_ICON_TOTAL_STEPS);
        let divisor = total_duration.as_nanos().max(1);
        value.div_ceil(divisor) as u32
    }
}
