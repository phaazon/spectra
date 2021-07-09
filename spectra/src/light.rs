pub trait Light {
  type Color;

  fn color(&self) -> &Self::Color;
}

pub trait DirLight: Light {
  type Dir;

  fn dir(&self) -> &Self::Dir;
}

#[cfg(test)]
pub mod tests {
  use super::*;

  #[derive(Debug)]
  pub struct LightColor([u8; 3]);

  impl LightColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
      LightColor([r, g, b])
    }
  }

  #[derive(Debug)]
  pub struct DirLightImpl {
    color: LightColor,
    dir: [f32; 3],
  }

  impl DirLightImpl {
    pub fn new(color: LightColor, dir: [f32; 3]) -> Self {
      Self { color, dir }
    }
  }

  impl Light for DirLightImpl {
    type Color = LightColor;

    fn color(&self) -> &Self::Color {
      &self.color
    }
  }

  impl DirLight for DirLightImpl {
    type Dir = [f32; 3];

    fn dir(&self) -> &Self::Dir {
      &self.dir
    }
  }
}
