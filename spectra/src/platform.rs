use crate::{
  renderer::Frame,
  resource::{Name, Resource},
};

#[derive(Debug)]
pub enum NextStep<T> {
  Exit,
  Continue(T),
}

impl<T> From<T> for NextStep<T> {
  fn from(t: T) -> Self {
    Self::Continue(t)
  }
}

pub trait Platform {
  type Store;
  type Err;
}

pub trait PlatformFetch<R>: Platform
where
  R: Resource,
{
  fn fetch(&mut self, name: impl Name<Resource = R>) -> Result<R::ResourceID, Self::Err>;
}

pub trait App<P>: Sized {
  type Input;
  type Frame: Frame;
  type Err;

  fn bootstrap(platform: &mut P) -> Result<Self, Self::Err>;
  fn step(self, platform: &mut P, inputs: impl Iterator<Item = Self::Input>) -> NextStep<Self>;
  fn render(&mut self, frame: &mut Self::Frame);
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::{
    entity::tests::{EntityImpl, MaterialImpl},
    light::tests::{DirLightImpl, LightColor},
    renderer::{DefaultFrame, Frame},
    resource::tests::{StoreImpl, TestName, TestResourceID},
  };

  pub struct AppImpl {
    _material: TestResourceID<MaterialImpl>,
    _dir_light: TestResourceID<DirLightImpl>,
  }

  impl<P> App<P> for AppImpl
  where
    P: Platform<Store = StoreImpl, Err = ()>
      + PlatformFetch<MaterialImpl>
      + PlatformFetch<DirLightImpl>,
  {
    type Input = ();

    type Frame = DefaultFrame<EntityImpl, DirLightImpl>;

    type Err = ();

    fn bootstrap(platform: &mut P) -> Result<Self, Self::Err> {
      let _material = platform.fetch(TestName::material("material.mat"))?;
      let _dir_light = platform.fetch(TestName::dir_light("dir_light.json"))?;
      let app = AppImpl {
        _material,
        _dir_light,
      };

      Ok(app)
    }

    fn step(self, _platform: &mut P, _inputs: impl Iterator<Item = Self::Input>) -> NextStep<Self> {
      NextStep::Exit
    }

    fn render(&mut self, frame: &mut Self::Frame) {
      frame.dir_light(DirLightImpl::new(
        LightColor::new(127, 127, 127),
        [0., -1., 0.],
      ));
    }
  }
}
