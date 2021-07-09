//! Camera related features.

use cgmath::{
  perspective, InnerSpace as _, Matrix4, One as _, Point3, Quaternion, Rad, Rotation as _,
  Rotation3 as _, Vector3,
};
use std::f32::consts::PI;

/// A freefly camera.
#[derive(Clone, Debug, PartialEq)]
pub struct FreeflyCamera {
  /// Aspect ratio (width / height) of the viewport.
  aspect_ratio: f32,
  /// Vertical field of view.
  fovy: Rad<f32>,
  /// Z-near clipping distance.
  z_near: f32,
  /// Z-far clipping distance.
  z_far: f32,
  /// Position of the camera.
  position: Point3<f32>,
  /// Orientation angle around the X axis.
  x_orientation_theta: Rad<f32>,
  /// Orientation angle around the Y axis.
  y_orientation_theta: Rad<f32>,
  /// Orientation of the camera.
  orientation: Quaternion<f32>,
  /// Projection * view matrix.
  projview: Matrix4<f32>,
}

impl FreeflyCamera {
  /// Create a new freefly camera for the given aspect ratio and fovy.
  pub fn new(aspect_ratio: f32, fovy: impl Into<Rad<f32>>, z_near: f32, z_far: f32) -> Self {
    let fovy = fovy.into();

    Self {
      aspect_ratio,
      fovy,
      z_near,
      z_far,
      position: Point3::new(0., 0., 0.),
      x_orientation_theta: Rad(0.),
      y_orientation_theta: Rad(0.),
      orientation: Quaternion::from_angle_y(Rad(0.)),
      projview: Matrix4::one(),
    }
  }

  /// Aspect ratio.
  pub fn aspect_ratio(&self) -> f32 {
    self.aspect_ratio
  }

  pub fn field_of_view(&self) -> Rad<f32> {
    self.fovy
  }

  pub fn z_near(&self) -> f32 {
    self.z_near
  }

  pub fn z_far(&self) -> f32 {
    self.z_far
  }

  pub fn position(&self) -> &Point3<f32> {
    &self.position
  }

  pub fn orientation(&self) -> &Quaternion<f32> {
    &self.orientation
  }

  pub fn projection_view(&self) -> &Matrix4<f32> {
    &self.projview
  }

  pub fn view(&self) -> Matrix4<f32> {
    Matrix4::from(self.orientation)
      * Matrix4::from_translation(-self.position.to_homogeneous().truncate())
  }

  /// Recompute the projection view matrix.
  fn recompute_projview(&mut self) {
    let qy = Quaternion::from_angle_y(self.y_orientation_theta);
    let qx = Quaternion::from_angle_x(self.x_orientation_theta);

    // Orientation of the camera. Used for both the skybox (by inverting it) and the cube.
    self.orientation = (qx * qy).normalize();

    // Projection.
    let projection = perspective(self.fovy, self.aspect_ratio, self.z_near, self.z_far);
    self.projview = projection * self.view();
  }

  /// Change the aspect ratio.
  pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
    self.aspect_ratio = aspect_ratio;
    self.recompute_projview();
  }

  /// Change the vertical field of view.
  pub fn set_field_of_view(&mut self, fovy: impl Into<Rad<f32>>) {
    let Rad(fovy) = fovy.into();

    self.fovy = Rad(fovy.max(0.).min(PI - f32::EPSILON));
    self.recompute_projview();
  }

  /// Change the Z-near clipping distance.
  pub fn set_z_near(&mut self, z_near: f32) {
    self.z_near = z_near;
    self.recompute_projview();
  }

  /// Change the Z-far clipping distance.
  pub fn set_z_far(&mut self, z_far: f32) {
    self.z_far = z_far;
    self.recompute_projview();
  }

  /// Move the camera by the given vector.
  pub fn move_by(&mut self, v: Vector3<f32>) {
    self.position -= self.orientation.invert().rotate_vector(v);
    self.recompute_projview();
  }

  /// Change the orientation with relative angle offsets on X and Y.
  pub fn orient(&mut self, x_theta: impl Into<Rad<f32>>, y_theta: impl Into<Rad<f32>>) {
    let x_theta = x_theta.into();
    let y_theta = y_theta.into();

    self.x_orientation_theta += x_theta;
    self.y_orientation_theta += y_theta;
    self.recompute_projview();
  }
}
