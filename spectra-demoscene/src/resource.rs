use serde::{Deserialize, Serialize};
use spectra::resource::{Compute, Get, Name, Resource, ID};
use splines::{Interpolate, Spline};
use std::{collections::HashMap, hash::Hash, marker::PhantomData, mem};

use crate::{
  light::{DirLight, DirLightSource},
  material::{ColorMaterial, ColorMaterialSource},
  obj::Obj,
  time::Time,
  transform::{Transform, TransformSource},
};

/// A scarce resource handle.
///
/// Resource handles are provided by
#[derive(Debug)]
pub struct ResourceID<T> {
  id: usize,
  _phantom: PhantomData<*const T>,
}

impl<T> Clone for ResourceID<T> {
  fn clone(&self) -> Self {
    ResourceID {
      id: self.id,
      _phantom: PhantomData,
    }
  }
}

impl<T> Copy for ResourceID<T> {}

impl<T> PartialEq for ResourceID<T> {
  fn eq(&self, other: &Self) -> bool {
    self.id.eq(&other.id)
  }
}

impl<T> Eq for ResourceID<T> {}

impl<T> Hash for ResourceID<T> {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.id.hash(state)
  }
}

impl<T> ResourceID<T> {
  fn new(id: usize) -> Self {
    Self {
      id,
      _phantom: PhantomData,
    }
  }
}

impl<T> ID<T> for ResourceID<T>
where
  T: Resource<ResourceID = ResourceID<T>>,
{
  fn get_id(&self) -> &T::ResourceID {
    self
  }
}

pub struct ResourceName<T> {
  name: String,
  _phantom: PhantomData<*const T>,
}

impl<T> AsRef<str> for ResourceName<T> {
  fn as_ref(&self) -> &str {
    &self.name
  }
}

impl<T> Into<String> for ResourceName<T> {
  fn into(self) -> String {
    self.name
  }
}

impl<T> From<String> for ResourceName<T> {
  fn from(name: String) -> Self {
    Self {
      name,
      _phantom: PhantomData,
    }
  }
}

impl<T> Name for ResourceName<T> {
  type Resource = T;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Parameter<T> {
  /// Time-varying parameter.
  Varying(Spline<f32, T>),

  /// Constant value that doesn’t change over time.
  Const(T),
}

impl<T> Default for Parameter<T>
where
  T: Default,
{
  fn default() -> Self {
    Self::Const(T::default())
  }
}

impl<T> Parameter<T>
where
  T: Interpolate<f32>,
{
  /// Compute the parameter at the given [`Time`].
  pub fn compute(&self, t: Time) -> Option<T> {
    match self {
      Parameter::Varying(spline) => spline.clamped_sample(t.to_secs()),
      Parameter::Const(x) => Some(*x),
    }
  }

  /// Compute the parameter at the given [`Time`].
  pub fn compute_or_default(&self, t: Time) -> T
  where
    T: Default,
  {
    match self {
      Parameter::Varying(spline) => spline.clamped_sample(t.to_secs()).unwrap_or_default(),
      Parameter::Const(x) => *x,
    }
  }
}

macro_rules! impl_Store {
  (get $($r:tt)*) => {
  };

  (compute $($r:tt)*) => {
  };

  ($( ( resource = $t:ty, source = $source:ty, container = $res_container:ident, mappings = $res_mappings:ident $(,)?) ),* $(,)?) => {
    /// A scarce resource store.
    ///
    /// This kind of store holds runtime resources, which have names and “handles”, allowing for quick lookups. There is a
    /// bijective mapping between resource names and resource handles.
    #[derive(Debug)]
    pub struct Store {
      // metadata
      next_id: usize,

      $(
        $res_container: HashMap<usize, $source>,
        $res_mappings: HashMap<String, usize>,
      )*
    }

    impl Default for Store {
      fn default() -> Self {
        Self::new()
      }
    }

    impl Store {
      /// Create an empty [`Store`].
      pub fn new() -> Self {
        Store {
          next_id: 0,
          $(
            $res_container: HashMap::new(),
            $res_mappings: HashMap::new(),
          )*
        }
      }
    }

    $(
      impl spectra::resource::Store<$t> for Store {
        fn register(&mut self, name: impl Name<Resource = $t>, source: $source) -> ResourceID<$t> {
          let id = self.next_id;
          self.next_id += 1;

          self.$res_container.insert(id, source);
          self.$res_mappings.insert(name.into(), id);

          ResourceID::new(id)
        }

        fn update(&mut self, id: &impl ID<$t>, source: $source) -> Option<$source> {
          self
            .$res_container
            .get_mut(&id.get_id().id)
            .map(|present| mem::replace(present, source))
        }

        fn translate(&self, name: impl Name<Resource = $t>) -> Option<ResourceID<$t>> {
          self
            .$res_mappings
            .get(name.as_ref())
            .map(|&id| ResourceID::new(id))
        }
      }
    )*
  };
}

macro_rules! impl_Get {
  ($t:ty, $container:ident) => {
    impl Get<$t> for Store {
      fn get(&self, id: &ResourceID<$t>) -> Option<&$t> {
        self.$container.get(&id.id)
      }
    }
  };
}

macro_rules! impl_Compute {
  ($t:ty, $container:ident, $($field:ident),* $(,)?) => {
    impl Compute<$t> for Store {
      type Time = Time;

      fn compute(&self, t: Self::Time, id: &ResourceID<$t>) -> Option<$t> {
        let source = self.$container.get(&id.id)?;
        $(
          let $field = source.$field.compute(t)?;
        )*
        let x = <$t>::new($($field),*);
        Some(x)
      }
    }
  };
}

impl_Store!(
  (
    resource = Obj,
    source = Obj,
    container = objs,
    mappings = objs_mappings,
  ),
  (
    resource = ColorMaterial,
    source = ColorMaterialSource,
    container = color_materials,
    mappings = color_materials_mappings,
  ),
  (
    resource = Transform,
    source = TransformSource,
    container = transforms,
    mappings = transforms_mappings,
  ),
  (
    resource = DirLight,
    source = DirLightSource,
    container = dir_lights,
    mappings = dir_lights_mappings,
  ),
);

impl_Get!(Obj, objs);

impl_Compute!(
  ColorMaterial,
  color_materials,
  ambient,
  diffuse,
  specular,
  shininess
);
impl_Compute!(Transform, transforms, position, orientation, scale);
impl_Compute!(DirLight, dir_lights, color, dir, scattering, power);
