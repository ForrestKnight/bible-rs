//! Interface motion: a critically damped spring for panels that slide in and out.

/// How stiff the spring is (1/s). From rest it covers 95 % of the way in 135 ms and 99 % in
/// 190 ms, and is within a thousandth of the target (a quarter point across the sidebar) at
/// 265 ms, where it stops: the quick start and long, soft landing of AppKit's sidebar.
const OMEGA: f64 = 35.0;
/// Closer than this to the target, and slower than [`SETTLE_SPEED`], the spring rests there.
const SETTLE: f64 = 1e-3;
const SETTLE_SPEED: f64 = 0.05;

/// A value between 0 and 1 that follows its target as a critically damped spring.
///
/// Evaluated in closed form from the frame's timestamp, so a late frame lands where it would
/// have been rather than slowing the motion down, and retargeted mid-flight from its current
/// position and velocity, so reversing it halfway turns smoothly instead of jumping.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    target: f64,
    /// Position and velocity when the current motion started, and when (seconds).
    from: f64,
    velocity: f64,
    start: f64,
    moving: bool,
}

impl Spring {
    /// A spring resting at `value`.
    pub fn new(value: f32) -> Self {
        let v = value as f64;
        Self { target: v, from: v, velocity: 0.0, start: 0.0, moving: false }
    }

    /// Heads for `target` and returns the value at time `now`. A new target counts from `since`,
    /// the frame that changed it, so the first frame drawn after a click already moves.
    /// `instant` jumps straight there instead (Reduce Motion).
    pub fn update(&mut self, target: f32, since: f64, now: f64, instant: bool) -> f32 {
        let target = target as f64;
        if target != self.target {
            let since = since.min(now);
            let (x, v) = self.at(since);
            *self = Self { target, from: x, velocity: v, start: since, moving: !instant };
        }
        if self.moving {
            let (x, v) = self.at(now);
            if (x - target).abs() < SETTLE && v.abs() < SETTLE_SPEED {
                self.moving = false;
            }
        }
        self.at(now).0 as f32
    }

    /// Whether it is still on its way (the caller keeps repainting until it isn't).
    pub fn moving(&self) -> bool {
        self.moving
    }

    pub fn target(&self) -> f32 {
        self.target as f32
    }

    /// Position and velocity at `now`: x(t) = target + (c1 + c2·t)·e^(−ωt).
    fn at(&self, now: f64) -> (f64, f64) {
        if !self.moving {
            return (self.target, 0.0);
        }
        let t = (now - self.start).max(0.0);
        let c1 = self.from - self.target;
        let c2 = self.velocity + OMEGA * c1;
        let e = (-OMEGA * t).exp();
        (self.target + (c1 + c2 * t) * e, (c2 - OMEGA * (c1 + c2 * t)) * e)
    }
}

/// Whether the system asks for less motion (macOS: Accessibility › Display › Reduce motion).
#[cfg(target_os = "macos")]
pub fn system_reduces_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// Whether the system asks for less motion (Windows: Accessibility › Visual effects ›
/// Animation effects off).
#[cfg(windows)]
pub fn system_reduces_motion() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW};
    let mut animate: windows_sys::core::BOOL = 1;
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL into `animate`.
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut animate as *mut i32).cast(), 0) };
    ok != 0 && animate == 0
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn system_reduces_motion() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::Spring;

    const FRAME: f64 = 1.0 / 120.0;

    #[test]
    fn rests_until_the_target_changes() {
        let mut s = Spring::new(0.0);
        assert_eq!(s.update(0.0, 5.0, 5.0, false), 0.0);
        assert!(!s.moving());
    }

    #[test]
    fn opens_monotonically_and_settles_exactly() {
        let mut s = Spring::new(0.0);
        let mut t = 10.0;
        let mut last = s.update(1.0, t - FRAME, t, false);
        assert!(last > 0.0, "the first frame after the click already moves");
        while s.moving() {
            t += FRAME;
            let x = s.update(1.0, t, t, false);
            assert!(x >= last && x <= 1.0, "{x} after {last}");
            last = x;
            assert!(t - 10.0 < 0.3, "still moving after 300 ms");
        }
        assert_eq!(last, 1.0);
        assert!(t - 10.0 > 0.2, "settled too soon: {} s", t - 10.0);
    }

    #[test]
    fn reverses_without_jumping() {
        let mut s = Spring::new(0.0);
        s.update(1.0, 0.0, 0.0, false);
        let before = s.update(1.0, 0.06, 0.06, false);
        let after = s.update(0.0, 0.06, 0.06, false);
        assert_eq!(before, after, "retargeting keeps the position");
        let next = s.update(0.0, 0.06 + FRAME, 0.06 + FRAME, false);
        // Still moving forward for a moment: the velocity carries over.
        assert!(next > after - 1e-3 && next < after + 0.05);
    }

    #[test]
    fn reduce_motion_jumps() {
        let mut s = Spring::new(1.0);
        assert_eq!(s.update(0.0, 1.0, 1.0, true), 0.0);
        assert!(!s.moving());
    }

    #[test]
    fn a_late_frame_lands_where_it_should() {
        let (mut a, mut b) = (Spring::new(0.0), Spring::new(0.0));
        a.update(1.0, 0.0, 0.0, false);
        b.update(1.0, 0.0, 0.0, false);
        for i in 1..=6 {
            a.update(1.0, 0.0, i as f64 * FRAME, false);
        }
        assert_eq!(a.update(1.0, 0.0, 0.05, false), b.update(1.0, 0.0, 0.05, false));
    }
}
