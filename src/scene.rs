use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;

use anim::Cont;
use entity::Entity;
use model::Model;
use resource::ResourceManager;

/// A typed identifier.
#[derive(Debug)]
pub struct Id<'a, T> where T: 'a {
  pub id: u32,
  _t: PhantomData<&'a T>
}

impl<'a, T> Id<'a, T> {
  pub fn new(id: u32) -> Self {
    Id {
      id: id,
      _t: PhantomData
    }
  }
}

impl<'a, T> Clone for Id<'a, T> {
  fn clone(&self) -> Self {
    self.id.into()
  }
}

impl<'a, T> Deref for Id<'a, T> {
  type Target = u32;

  fn deref(&self) -> &Self::Target {
    &self.id
  }
}

impl<'a, T> From<u32> for Id<'a, T> {
  fn from(id: u32) -> Self {
    Id::new(id)
  }
}

/// The scene type.
///
/// This type gathers all the required objects a scene needs to correctly handle and render all
/// visual effects.
pub struct Scene<'a> {
  /// Resource manager; used to handle scarce resources.
  res_manager: ResourceManager,
  /// All models used in the scene.
  models: Vec<Model>,
  /// Model cache used to resolve Id based on instance name.
  model_cache: HashMap<String, Id<'a, Model>>,
  /// Model entities used in the scene, containing `Id` to the models of the scene.
  model_entities: HashMap<String, SceneModelEntity<'a>>
}

impl<'a> Scene<'a> {
  pub fn new<P>(root: P) -> Self where P: AsRef<Path>{
    Scene {
      res_manager: ResourceManager::new(root),
      models: Vec::new(),
      model_cache: HashMap::new(),
      model_entities: HashMap::new()
    }
  }

  pub fn resource_manager(&mut self) -> &mut ResourceManager {
    &mut self.res_manager
  }
}

/// An model entity living in a scene.
///
/// It can either be a static model entity, in which case it just holds a transform object or it can
/// be a dynamic model entity, which holds a continuous model entity you can sample in time to
/// retrieve the varying transform.
pub enum SceneModelEntity<'a> {
  Static(Entity<Id<'a, Model>>),
  Dynamic(Cont<'a, Entity<Id<'a, Model>>>)
}

pub trait Get<T>: Sized {
  type Id;

  fn get_id(&mut self, name: &str) -> Option<Self::Id>;
  fn get_by_id(&self, id: Self::Id) -> Option<&T>;
  fn get(&mut self, name: &str) -> Option<&T> {
    self.get_id(name).and_then(move |i| self.get_by_id(i))
  }
}

impl<'a> Get<Model> for Scene<'a> {
  type Id = Id<'a, Model>;

  fn get_id(&mut self, name: &str) -> Option<Self::Id> {
    match self.model_cache.get(name).cloned() {
      id@Some(..) => {
        deb!("cache hit for model \"{}\"", name);
        id
      },
      None => {
        // cache miss; load then
        let path_str = format!("data/models/{}.obj", name);
        let path = Path::new(&path_str);

        deb!("cache miss for model \"{}\"", name);

        if path.exists() {
          match Model::load(&mut self.res_manager, path) {
            Ok(model) => {
              let model_id: Id<Model> = (self.models.len() as u32).into();

              // add the model to the list of loaded models
              self.models.push(model);
              // add the model to the cache
              self.model_cache.insert(name.to_owned(), model_id.clone());

              Some(model_id)
            },
            Err(e) => {
              err!("unable to load model '{}': '{:?}'", name, e);
              None
            }
          }
        } else { // TODO: add a manifest override to look in somewhere else
          err!("model '{}' cannot be found", name);
          None
        }
      }
    }
  }

  fn get_by_id(&self, id: Self::Id) -> Option<&Model> {
    self.models.get(*id as usize)
  }
}
