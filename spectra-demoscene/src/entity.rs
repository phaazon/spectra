use crate::{material::ColorMaterial, model::Model, transform::Transform};

#[derive(Debug)]
pub struct Entity {
  pub model: Model,
  pub transform: Transform,
  pub material: ColorMaterial,
}

impl Entity {
  pub fn new(model: Model, transform: Transform, material: ColorMaterial) -> Self {
    Self {
      model,
      transform,
      material,
    }
  }
}

impl spectra::entity::Entity for Entity {
  type Model = Model;
  type Transform = Transform;
  type Material = ColorMaterial;

  fn model(&self) -> &Self::Model {
    &self.model
  }

  fn transform(&self) -> &Self::Transform {
    &self.transform
  }

  fn material(&self) -> &Self::Material {
    &self.material
  }
}
