//! Materials materials materials!

use crate::resource::{Parameter, ResourceID};
use cgmath::Vector3;
use serde::{Deserialize, Serialize};
use spectra::resource::Resource;

/// Opaque material, defining entities that can have a diffuse and specular absorption values.
#[derive(Clone, Debug)]
pub struct ColorMaterial {
  pub ambient: Vector3<f32>,
  pub diffuse: Vector3<f32>,
  pub specular: Vector3<f32>,
  pub shininess: f32,
}

impl ColorMaterial {
  pub fn new(
    ambient: Vector3<f32>,
    diffuse: Vector3<f32>,
    specular: Vector3<f32>,
    shininess: f32,
  ) -> Self {
    Self {
      ambient,
      diffuse,
      specular,
      shininess,
    }
  }
}

impl Default for ColorMaterial {
  fn default() -> Self {
    Self::new(
      Vector3::new(0.01, 0.01, 0.01),
      Vector3::new(0.6, 0.6, 0.6),
      Vector3::new(1., 1., 1.),
      32.,
    )
  }
}

/// Source representation of a [`ColorMaterial`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ColorMaterialSource {
  pub ambient: Parameter<Vector3<f32>>,
  pub diffuse: Parameter<Vector3<f32>>,
  pub specular: Parameter<Vector3<f32>>,
  pub shininess: Parameter<f32>,
}

impl Default for ColorMaterialSource {
  fn default() -> Self {
    let mat = ColorMaterial::default();
    let ambient = Parameter::Const(mat.ambient);
    let diffuse = Parameter::Const(mat.diffuse);
    let specular = Parameter::Const(mat.specular);
    let shininess = Parameter::Const(mat.shininess);

    ColorMaterialSource {
      ambient,
      diffuse,
      specular,
      shininess,
    }
  }
}

impl Resource for ColorMaterial {
  type Source = ColorMaterialSource;
  type ResourceID = ResourceID<ColorMaterial>;
}
