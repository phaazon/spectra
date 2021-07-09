use crate::{entity::Entity, light::DirLight};

pub trait Frame {
  type Entity: Entity;
  type DirLight: DirLight;

  fn entity(&mut self, entity: Self::Entity);

  fn debug_show_origin(&mut self);

  fn dir_light(&mut self, light: Self::DirLight);
}

pub struct DefaultFrame<E, DL> {
  entities: Vec<E>,
  dir_light: Option<DL>,
  debug_show_origin: bool,
}

impl<E, DL> DefaultFrame<E, DL> {
  pub fn new() -> Self {
    Self {
      entities: Vec::new(),
      dir_light: None,
      debug_show_origin: false,
    }
  }

  pub fn reset(&mut self) {
    self.entities.clear();
    self.dir_light = None;
    self.debug_show_origin = false;
  }

  pub fn get_entities(&self) -> &[E] {
    &self.entities
  }

  pub fn get_dir_light(&self) -> Option<&DL> {
    self.dir_light.as_ref()
  }

  pub fn get_debug_show_origin(&self) -> bool {
    self.debug_show_origin
  }
}

impl<E, DL> Frame for DefaultFrame<E, DL>
where
  E: Entity,
  DL: DirLight,
{
  type Entity = E;
  type DirLight = DL;

  fn entity(&mut self, entity: Self::Entity) {
    self.entities.push(entity);
  }

  fn debug_show_origin(&mut self) {
    self.debug_show_origin = true;
  }

  fn dir_light(&mut self, light: Self::DirLight) {
    self.dir_light = Some(light);
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::{
    entity::tests::{EntityImpl, MaterialImpl, ModelImpl, TransformImpl},
    light::tests::{DirLightImpl, LightColor},
  };

  #[test]
  fn scene_description_test() {
    let mut frame = DefaultFrame::new();
    frame.entity(EntityImpl::new(
      ModelImpl::Sphere,
      TransformImpl::Rotation,
      MaterialImpl::Red,
    ));

    frame.debug_show_origin();

    frame.dir_light(DirLightImpl::new(LightColor::new(255, 0, 0), [0., -1., 0.]));
  }
}
