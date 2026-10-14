use std::fmt;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use super::error::EditError;
use crate::media::metadata::Rational;

/// Explicit rounding policy for discrete frame, sample, and PTS boundary conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundMode {
    /// Truncate toward negative infinity (floor).
    Floor,
    /// Round toward positive infinity (ceil).
    Ceil,
    /// Round to nearest integer with half rounded away from zero.
    #[default]
    Nearest,
}

/// Canonical signed 64-bit integer microsecond time unit (`i64`).
///
/// In OpenTake, all editing positions, durations, and interval endpoints are
/// represented in integer microseconds:
/// $$1\text{ second} = 1{,}000{,}000\,\mu\text{s}$$
///
/// Floating-point numbers are strictly forbidden as domain time values.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MediaTimeUs(pub i64);

impl MediaTimeUs {
    pub const ZERO: Self = Self(0);
    pub const MILLISECOND: Self = Self(1_000);
    pub const SECOND: Self = Self(1_000_000);
    pub const MINUTE: Self = Self(60_000_000);
    pub const HOUR: Self = Self(3_600_000_000);
    pub const MAX: Self = Self(i64::MAX);

    /// Constructs a `MediaTimeUs` from raw microseconds.
    #[inline]
    pub const fn from_micros(us: i64) -> Self {
        Self(us)
    }

    /// Returns the underlying microsecond value as an `i64`.
    #[inline]
    pub const fn as_micros(self) -> i64 {
        self.0
    }

    /// Validates that this timestamp is non-negative ($t \ge 0$).
    pub fn ensure_non_negative(self) -> Result<Self, EditError> {
        if self.0 < 0 {
            Err(EditError::NegativeTime { time_us: self.0 })
        } else {
            Ok(self)
        }
    }

    /// Checks if the time value is non-negative ($t \ge 0$).
    #[inline]
    pub const fn is_non_negative(self) -> bool {
        self.0 >= 0
    }

    /// Checks if the time value is strictly positive ($t > 0$).
    #[inline]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Checked addition with integer overflow protection.
    pub fn checked_add(self, rhs: Self) -> Result<Self, EditError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(EditError::ArithmeticOverflow)
    }

    /// Checked subtraction with integer overflow protection.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, EditError> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(EditError::ArithmeticOverflow)
    }

    /// Checked multiplication by an integer scalar.
    pub fn checked_mul(self, scalar: i64) -> Result<Self, EditError> {
        self.0
            .checked_mul(scalar)
            .map(Self)
            .ok_or(EditError::ArithmeticOverflow)
    }

    /// Checked division by an integer scalar.
    pub fn checked_div(self, divisor: i64) -> Result<Self, EditError> {
        if divisor == 0 {
            return Err(EditError::ArithmeticOverflow);
        }
        self.0
            .checked_div(divisor)
            .map(Self)
            .ok_or(EditError::ArithmeticOverflow)
    }

    /// Saturating addition.
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Lossy conversion to floating-point seconds.
    ///
    /// **WARNING:** Use exclusively for UI display or WebKit media player time assignments.
    /// Never use this value as authoritative timeline calculation truth!
    pub fn to_seconds_f64_lossy(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    /// Converts from floating-point seconds with explicit rounding.
    pub fn from_seconds_f64(secs: f64, round: RoundMode) -> Result<Self, EditError> {
        if !secs.is_finite() {
            return Err(EditError::ArithmeticOverflow);
        }
        let micros_f = secs * 1_000_000.0;
        let rounded = match round {
            RoundMode::Floor => micros_f.floor(),
            RoundMode::Ceil => micros_f.ceil(),
            RoundMode::Nearest => micros_f.round(),
        };
        if rounded < (i64::MIN as f64) || rounded > (i64::MAX as f64) {
            return Err(EditError::ArithmeticOverflow);
        }
        Ok(Self(rounded as i64))
    }

    // =========================================================================
    // Rational Timebase, Frame Rate & Audio Sample Conversions
    // =========================================================================

    /// Computes the exact timestamp corresponding to a discrete video frame index:
    /// $$t = \frac{\text{frame} \times 1{,}000{,}000 \times \text{den}}{\text{num}}$$
    /// Uses 128-bit checked intermediate arithmetic to eliminate multiplication overflow.
    pub fn from_frame(
        frame_index: i64,
        fps: Rational,
        round: RoundMode,
    ) -> Result<Self, EditError> {
        if fps.num == 0 || fps.den == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        let num_128 = (frame_index as i128)
            .checked_mul(1_000_000)
            .and_then(|v| v.checked_mul(fps.den as i128))
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = fps.num as i128;
        let us = div_round_i128(num_128, den_128, round)?;
        i64::try_from(us)
            .map(Self)
            .map_err(|_| EditError::ArithmeticOverflow)
    }

    /// Converts this microsecond timestamp to a discrete video frame index:
    /// $$\text{frame} = \frac{t \times \text{num}}{\text{den} \times 1{,}000{,}000}$$
    /// Uses 128-bit checked intermediate arithmetic.
    pub fn to_frame(self, fps: Rational, round: RoundMode) -> Result<i64, EditError> {
        if fps.num == 0 || fps.den == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        let num_128 = (self.0 as i128)
            .checked_mul(fps.num as i128)
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = (fps.den as i128)
            .checked_mul(1_000_000)
            .ok_or(EditError::ArithmeticOverflow)?;

        let frame_128 = div_round_i128(num_128, den_128, round)?;
        i64::try_from(frame_128).map_err(|_| EditError::ArithmeticOverflow)
    }

    /// Snaps this timestamp to the nearest video frame boundary.
    pub fn snap_to_frame(self, fps: Rational, round: RoundMode) -> Result<Self, EditError> {
        let frame = self.to_frame(fps, round)?;
        Self::from_frame(frame, fps, round)
    }

    /// Computes the exact timestamp corresponding to an audio sample index:
    /// $$t = \frac{\text{sample} \times 1{,}000{,}000}{\text{sample\_rate}}$$
    pub fn from_sample(
        sample_index: i64,
        sample_rate: u32,
        round: RoundMode,
    ) -> Result<Self, EditError> {
        if sample_rate == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        let num_128 = (sample_index as i128)
            .checked_mul(1_000_000)
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = sample_rate as i128;
        let us = div_round_i128(num_128, den_128, round)?;
        i64::try_from(us)
            .map(Self)
            .map_err(|_| EditError::ArithmeticOverflow)
    }

    /// Converts this timestamp to an audio sample index.
    pub fn to_sample(self, sample_rate: u32, round: RoundMode) -> Result<i64, EditError> {
        if sample_rate == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        let num_128 = (self.0 as i128)
            .checked_mul(sample_rate as i128)
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = 1_000_000i128;
        let sample_128 = div_round_i128(num_128, den_128, round)?;
        i64::try_from(sample_128).map_err(|_| EditError::ArithmeticOverflow)
    }

    /// Snaps this timestamp to the nearest audio sample boundary.
    pub fn snap_to_sample(self, sample_rate: u32, round: RoundMode) -> Result<Self, EditError> {
        let sample = self.to_sample(sample_rate, round)?;
        Self::from_sample(sample, sample_rate, round)
    }

    /// Converts an FFmpeg Presentation Timestamp (PTS) in a given rational stream timebase
    /// to canonical microseconds.
    pub fn from_pts(pts: i64, timebase: Rational, round: RoundMode) -> Result<Self, EditError> {
        if timebase.num == 0 || timebase.den == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        // pts * timebase.num / timebase.den seconds -> microseconds = pts * timebase.num * 1_000_000 / timebase.den
        let num_128 = (pts as i128)
            .checked_mul(timebase.num as i128)
            .and_then(|v| v.checked_mul(1_000_000))
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = timebase.den as i128;
        let us = div_round_i128(num_128, den_128, round)?;
        i64::try_from(us)
            .map(Self)
            .map_err(|_| EditError::ArithmeticOverflow)
    }

    /// Converts canonical microseconds to FFmpeg PTS in the given rational stream timebase.
    pub fn to_pts(self, timebase: Rational, round: RoundMode) -> Result<i64, EditError> {
        if timebase.num == 0 || timebase.den == 0 {
            return Err(EditError::ArithmeticOverflow);
        }

        // us / 1_000_000 = pts * timebase.num / timebase.den
        // pts = us * timebase.den / (timebase.num * 1_000_000)
        let num_128 = (self.0 as i128)
            .checked_mul(timebase.den as i128)
            .ok_or(EditError::ArithmeticOverflow)?;

        let den_128 = (timebase.num as i128)
            .checked_mul(1_000_000)
            .ok_or(EditError::ArithmeticOverflow)?;

        let pts_128 = div_round_i128(num_128, den_128, round)?;
        i64::try_from(pts_128).map_err(|_| EditError::ArithmeticOverflow)
    }
}

/// Helper performing integer division on 128-bit integers with explicit rounding mode.
fn div_round_i128(num: i128, den: i128, round: RoundMode) -> Result<i128, EditError> {
    if den == 0 {
        return Err(EditError::ArithmeticOverflow);
    }

    let quotient = num / den;
    let remainder = num % den;

    if remainder == 0 {
        return Ok(quotient);
    }

    let same_sign = (num >= 0) == (den >= 0);

    match round {
        RoundMode::Floor => {
            if same_sign {
                Ok(quotient)
            } else {
                Ok(quotient - 1)
            }
        }
        RoundMode::Ceil => {
            if same_sign {
                Ok(quotient + 1)
            } else {
                Ok(quotient)
            }
        }
        RoundMode::Nearest => {
            let abs_rem = remainder.abs();
            let abs_den = den.abs();
            let double_rem = abs_rem
                .checked_mul(2)
                .ok_or(EditError::ArithmeticOverflow)?;

            if double_rem > abs_den {
                // Closer to next integer
                if same_sign {
                    Ok(quotient + 1)
                } else {
                    Ok(quotient - 1)
                }
            } else if double_rem == abs_den {
                // Exact half: round away from zero
                if same_sign {
                    Ok(quotient + 1)
                } else {
                    Ok(quotient - 1)
                }
            } else {
                Ok(quotient)
            }
        }
    }
}

impl fmt::Display for MediaTimeUs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} us", self.0)
    }
}

impl Add for MediaTimeUs {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("MediaTimeUs addition overflow")
    }
}

impl Sub for MediaTimeUs {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
            .expect("MediaTimeUs subtraction overflow")
    }
}

impl From<i64> for MediaTimeUs {
    #[inline]
    fn from(us: i64) -> Self {
        Self(us)
    }
}

impl From<MediaTimeUs> for i64 {
    #[inline]
    fn from(t: MediaTimeUs) -> Self {
        t.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_time_basics() {
        let t1 = MediaTimeUs::from_micros(1_000_000);
        let t2 = MediaTimeUs::from_micros(500_000);

        assert_eq!((t1 + t2).as_micros(), 1_500_000);
        assert_eq!((t1 - t2).as_micros(), 500_000);
        assert!(t1 > t2);
    }

    #[test]
    fn test_frame_conversions_24fps() {
        let fps = Rational::new(24, 1).unwrap();
        // Frame 0 -> 0 us
        assert_eq!(
            MediaTimeUs::from_frame(0, fps, RoundMode::Nearest).unwrap(),
            MediaTimeUs(0)
        );
        // Frame 1 -> 41667 us
        assert_eq!(
            MediaTimeUs::from_frame(1, fps, RoundMode::Nearest).unwrap(),
            MediaTimeUs(41_667)
        );
        assert_eq!(
            MediaTimeUs::from_frame(1, fps, RoundMode::Floor).unwrap(),
            MediaTimeUs(41_666)
        );
        // Frame 24 -> 1,000,000 us (exact)
        assert_eq!(
            MediaTimeUs::from_frame(24, fps, RoundMode::Nearest).unwrap(),
            MediaTimeUs::SECOND
        );

        // Reverse: 1_000_000 us -> frame 24
        assert_eq!(
            MediaTimeUs::SECOND
                .to_frame(fps, RoundMode::Nearest)
                .unwrap(),
            24
        );
    }

    #[test]
    fn test_ntsc_29_97_fps_precision() {
        let fps = Rational::new(30000, 1001).unwrap();
        // 30000 frames at 29.97002997... fps is exactly 1001 seconds = 1,001,000,000 us
        let t = MediaTimeUs::from_frame(30000, fps, RoundMode::Nearest).unwrap();
        assert_eq!(t.as_micros(), 1_001_000_000);

        let f = t.to_frame(fps, RoundMode::Nearest).unwrap();
        assert_eq!(f, 30000);
    }

    #[test]
    fn test_audio_sample_rate_conversions() {
        // 48 kHz
        let t48 = MediaTimeUs::from_sample(48000, 48000, RoundMode::Nearest).unwrap();
        assert_eq!(t48, MediaTimeUs::SECOND);
        assert_eq!(t48.to_sample(48000, RoundMode::Nearest).unwrap(), 48000);

        // 44.1 kHz
        let t44 = MediaTimeUs::from_sample(44100, 44100, RoundMode::Nearest).unwrap();
        assert_eq!(t44, MediaTimeUs::SECOND);
        assert_eq!(t44.to_sample(44100, RoundMode::Nearest).unwrap(), 44100);
    }

    #[test]
    fn test_arithmetic_overflow_guards() {
        let max_time = MediaTimeUs(i64::MAX);
        assert!(max_time.checked_add(MediaTimeUs(1)).is_err());
        assert!(MediaTimeUs(i64::MIN).checked_sub(MediaTimeUs(1)).is_err());
    }
}
