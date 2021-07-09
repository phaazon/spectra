use crate::resource::{Parameter, ResourceID};
use cgmath::{
  Deg, Matrix4, Quaternion, Rotation3 as _, Transform as _, Vector3, Vector4, Zero as _,
};
use serde::{Deserialize, Serialize};
use spectra::resource::Resource;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform(Matrix4<f32>);

impl Transform {
  pub fn new(position: Vector3<f32>, orientation: Vector4<f32>, scale: f32) -> Self {
    // scale first then create the “look at” matrix
    let mat = Matrix4::from_translation(position)
      .concat(&Quaternion::from_axis_angle(orientation.truncate(), Deg(orientation.w)).into())
      .concat(&Matrix4::from_scale(scale));
    Self(mat)
  }
}

/// Resource representation of a [`Tansform`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TransformSource {
  pub position: Parameter<Vector3<f32>>,
  pub orientation: Parameter<Vector4<f32>>,
  pub scale: Parameter<f32>,
}

impl Default for TransformSource {
  fn default() -> Self {
    let position = Parameter::Const(Vector3::zero());
    let orientation = Parameter::Const(Vector4::new(1., 0., 0., 0.));
    let scale = Parameter::Const(1.);

    TransformSource {
      position,
      orientation,
      scale,
    }
  }
}

impl Resource for Transform {
  type Source = TransformSource;
  type ResourceID = ResourceID<Transform>;
}
