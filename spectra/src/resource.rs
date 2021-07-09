pub trait Resource {
  /// Source data to buidl the resource from.
  type Source;

  /// Resource unique handle.
  ///
  /// A [`ResourceID`] is a handle to a resource, uniquely identifying that resource in the whole system.
  type ResourceID;
}

pub trait Name: AsRef<str> + Into<String> + From<String> {
  type Resource;
}

pub trait ID<R>
where
  R: Resource,
{
  fn get_id(&self) -> &R::ResourceID;
}

pub trait Store<R>
where
  R: Resource,
{
  fn register(&mut self, name: impl Name<Resource = R>, source: R::Source) -> R::ResourceID;

  fn update(&mut self, id: &impl ID<R>, source: R::Source) -> Option<R::Source>;

  fn translate(&self, name: impl Name<Resource = R>) -> Option<R::ResourceID>;
}

pub trait Compute<R>
where
  R: Resource,
{
  type Time;

  fn compute(&self, time: Self::Time, id: &R::ResourceID) -> Option<R>;
}

pub trait Get<R>
where
  R: Resource,
{
  fn get(&self, id: &R::ResourceID) -> Option<&R>;
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::{entity::tests::MaterialImpl, light::tests::DirLightImpl};
  use std::{collections::HashMap, marker::PhantomData};

  #[derive(Debug)]
  pub enum TestResource {
    DirLight(DirLightImpl),
    Material(MaterialImpl),
  }

  impl From<DirLightImpl> for TestResource {
    fn from(a: DirLightImpl) -> Self {
      Self::DirLight(a)
    }
  }

  impl From<MaterialImpl> for TestResource {
    fn from(a: MaterialImpl) -> Self {
      Self::Material(a)
    }
  }

  pub trait FromTestResource: Sized {
    fn from_test_resource(r: TestResource) -> Option<Self>;
    fn from_test_resource_ref(r: &TestResource) -> Option<&Self>;
  }

  impl FromTestResource for DirLightImpl {
    fn from_test_resource(r: TestResource) -> Option<Self> {
      match r {
        TestResource::DirLight(l) => Some(l),
        _ => None,
      }
    }

    fn from_test_resource_ref(r: &TestResource) -> Option<&Self> {
      match r {
        TestResource::DirLight(ref l) => Some(l),
        _ => None,
      }
    }
  }

  impl FromTestResource for MaterialImpl {
    fn from_test_resource(r: TestResource) -> Option<Self> {
      match r {
        TestResource::Material(m) => Some(m),
        _ => None,
      }
    }

    fn from_test_resource_ref(r: &TestResource) -> Option<&Self> {
      match r {
        TestResource::Material(ref m) => Some(m),
        _ => None,
      }
    }
  }

  impl Resource for DirLightImpl {
    type Source = Self;
    type ResourceID = TestResourceID<Self>;
  }

  impl Resource for MaterialImpl {
    type Source = Self;
    type ResourceID = TestResourceID<Self>;
  }

  #[derive(Debug, Eq, PartialEq)]
  pub struct TestName<T> {
    name: String,
    _phantom: PhantomData<*const T>,
  }

  impl<T> Clone for TestName<T> {
    fn clone(&self) -> Self {
      Self {
        name: self.name.clone(),
        _phantom: PhantomData,
      }
    }
  }

  impl TestName<DirLightImpl> {
    pub fn dir_light(name: impl Into<String>) -> Self {
      Self {
        name: name.into(),
        _phantom: PhantomData,
      }
    }
  }

  impl TestName<MaterialImpl> {
    pub fn material(name: impl Into<String>) -> Self {
      Self {
        name: name.into(),
        _phantom: PhantomData,
      }
    }
  }

  impl<T> From<String> for TestName<T> {
    fn from(name: String) -> Self {
      Self {
        name,
        _phantom: PhantomData,
      }
    }
  }

  impl<T> Into<String> for TestName<T> {
    fn into(self) -> String {
      self.name
    }
  }

  impl<T> AsRef<str> for TestName<T> {
    fn as_ref(&self) -> &str {
      &self.name
    }
  }

  impl Name for TestName<DirLightImpl> {
    type Resource = DirLightImpl;
  }

  impl Name for TestName<MaterialImpl> {
    type Resource = MaterialImpl;
  }

  #[derive(Debug, Eq, PartialEq)]
  pub struct TestResourceID<T> {
    id: usize,
    _phantom: PhantomData<*const T>,
  }

  impl<T> Copy for TestResourceID<T> {}

  impl<T> Clone for TestResourceID<T> {
    fn clone(&self) -> Self {
      Self {
        id: self.id,
        _phantom: PhantomData,
      }
    }
  }

  impl<R> ID<R> for TestResourceID<R>
  where
    R: Resource<ResourceID = TestResourceID<R>>,
  {
    fn get_id(&self) -> &R::ResourceID {
      self
    }
  }

  impl<T> TestResourceID<T> {
    fn new(id: usize) -> Self {
      Self {
        id,
        _phantom: PhantomData,
      }
    }
  }

  #[derive(Debug)]
  pub struct StoreImpl {
    next_id: usize,
    resources: HashMap<usize, TestResource>,
    translations: HashMap<String, usize>,
  }

  impl StoreImpl {
    pub fn new() -> Self {
      StoreImpl {
        next_id: 0,
        resources: HashMap::new(),
        translations: HashMap::new(),
      }
    }
  }

  impl<R> Store<R> for StoreImpl
  where
    R: Resource<Source = R, ResourceID = TestResourceID<R>> + Into<TestResource> + FromTestResource,
  {
    fn register(&mut self, name: impl Name<Resource = R>, source: R::Source) -> TestResourceID<R> {
      let id = self.next_id;
      self.next_id += 1;

      self.translations.insert(name.into(), id);
      self.resources.insert(id, source.into());
      TestResourceID::new(id)
    }

    fn update(&mut self, id: &impl ID<R>, source: R::Source) -> Option<R::Source> {
      self
        .resources
        .insert(id.get_id().id, source.into())
        .and_then(R::from_test_resource)
    }

    fn translate(&self, name: impl Name<Resource = R>) -> Option<TestResourceID<R>> {
      self
        .translations
        .get(name.as_ref())
        .map(|&id| TestResourceID::new(id))
    }
  }

  impl<R> Get<R> for StoreImpl
  where
    R: Resource<Source = R, ResourceID = TestResourceID<R>> + FromTestResource,
  {
    fn get(&self, id: &TestResourceID<R>) -> Option<&R> {
      self
        .resources
        .get(&id.id)
        .and_then(R::from_test_resource_ref)
    }
  }

  #[test]
  fn declare_store_test() {
    let mut store = StoreImpl::new();
    let mat_name = TestName::material("foo");

    assert_eq!(store.translate(mat_name.clone()), None);

    let id = store.register(mat_name.clone(), MaterialImpl::Red);
    assert_eq!(store.translate(mat_name.clone()), Some(id));

    store.update(&id, MaterialImpl::Blue);
    assert_eq!(store.translate(mat_name.clone()), Some(id));
  }
}
