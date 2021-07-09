//! Supported light types.

use crate::resource::{Parameter, ResourceID};
use cgmath::Vector3;
use serde::{Deserialize, Serialize};
use spectra::resource::Resource;

/// Directional light.
#[derive(Clone, Debug)]
pub struct DirLight {
  pub color: Vector3<f32>,
  pub dir: Vector3<f32>,
  pub scattering: f32,
  pub power: f32,
}

impl DirLight {
  pub fn new(color: Vector3<f32>, dir: Vector3<f32>, scattering: f32, power: f32) -> Self {
    Self {
      color,
      dir,
      scattering,
      power,
    }
  }
}

impl Default for DirLight {
  fn default() -> Self {
    let color = Vector3::new(1., 1., 1.);
    let dir = Vector3::new(0., -1., 0.);
    let scattering = 1.;
    let power = 1.;

    Self::new(color, dir, scattering, power)
  }
}

/// Resource representation of [`Directional`] light.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct DirLightSource {
  pub color: Parameter<Vector3<f32>>,
  pub dir: Parameter<Vector3<f32>>,
  pub scattering: Parameter<f32>,
  pub power: Parameter<f32>,
}

impl Default for DirLightSource {
  fn default() -> Self {
    let light = DirLight::default();
    let color = Parameter::Const(light.color);
    let dir = Parameter::Const(light.dir);
    let scattering = Parameter::Const(light.scattering);
    let power = Parameter::Const(light.power);

    DirLightSource {
      color,
      dir,
      scattering,
      power,
    }
  }
}

impl Resource for DirLight {
  type Source = DirLightSource;
  type ResourceID = ResourceID<DirLight>;
}
