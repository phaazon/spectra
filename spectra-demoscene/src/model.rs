use crate::{obj::Obj, resource::ResourceID};
use cgmath::Matrix4;

#[derive(Clone, Debug)]
pub enum Model {
  /// A simple cube.
  Cube,

  /// A finite plane of 1×1, on the XY plane.
  FinitePlane,

  /// Arbitrary 3D object.
  OBJ { id: ResourceID<Obj> },
}

impl Model {
  /// Create a cube [`Model`] with a material.
  pub fn cube() -> Self {
    Model::Cube
  }

  /// Create a finite plane [`Model`] with a material.
  pub fn finite_plane() -> Self {
    Model::FinitePlane
  }

  /// Create an arbitrary object [`Model`] with a material.
  pub fn obj(id: ResourceID<Obj>) -> Self {
    Model::OBJ { id }
  }
}
