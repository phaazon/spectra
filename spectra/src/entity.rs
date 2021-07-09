pub trait Entity {
  type Model;
  type Transform;
  type Material;

  fn model(&self) -> &Self::Model;

  fn transform(&self) -> &Self::Transform;

  fn material(&self) -> &Self::Material;
}

#[cfg(test)]
pub mod tests {
  use super::*;

  #[derive(Debug, Eq, Hash, PartialEq)]
  pub enum ModelImpl {
    Cube,
    Sphere,
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  pub enum TransformImpl {
    Translation,
    Rotation,
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  pub enum MaterialImpl {
    Red,
    Green,
    Blue,
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  pub struct EntityImpl {
    model: ModelImpl,
    transform: TransformImpl,
    material: MaterialImpl,
  }

  impl EntityImpl {
    pub fn new(model: ModelImpl, transform: TransformImpl, material: MaterialImpl) -> Self {
      Self {
        model,
        transform,
        material,
      }
    }
  }

  impl Entity for EntityImpl {
    type Model = ModelImpl;
    type Transform = TransformImpl;
    type Material = MaterialImpl;

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

  #[test]
  fn entity_test() {
    let entity = EntityImpl::new(ModelImpl::Cube, TransformImpl::Rotation, MaterialImpl::Blue);

    assert_eq!(entity.model(), &ModelImpl::Cube);
    assert_eq!(entity.transform(), &TransformImpl::Rotation);
    assert_eq!(entity.material(), &MaterialImpl::Blue);
  }
}
