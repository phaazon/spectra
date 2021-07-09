use serde::{Deserialize, Serialize};
use splines::interpolate::Interpolator;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Time(f32);

impl Time {
  pub fn from_secs(secs: f32) -> Self {
    Time(secs)
  }

  pub fn to_secs(&self) -> f32 {
    self.0
  }
}

impl Interpolator for Time {
  fn normalize(self, start: Self, end: Self) -> Self {
    Time(self.0.normalize(start.0, end.0))
  }
}
