use crate::{
  resource::ResourceID,
  vertex::{TessIndex, TessVertex3, VNor, VPos3},
};
use core::fmt;
use spectra::resource::Resource;
use std::collections::HashMap;

/// Various metadata on an [`Obj`].
#[derive(Clone, Debug)]
pub struct ObjMetadata {
  /// Minimum position (i.e. lower part of the bounding box) on X, Y and Z axis.
  pub bb_min: [f32; 3],

  /// Maximum position (i.e. upper part of the bounding box) on X, Y and Z axis.
  pub bb_max: [f32; 3],
}

impl Default for ObjMetadata {
  fn default() -> Self {
    ObjMetadata {
      bb_min: [-1., -1., -1.],
      bb_max: [1., 1., 1.],
    }
  }
}

impl ObjMetadata {
  pub fn new() -> Self {
    Self {
      bb_min: [f32::MAX, f32::MAX, f32::MAX],
      bb_max: [f32::MIN, f32::MIN, f32::MIN],
    }
  }

  pub fn traverse_vertex(&mut self, [x, y, z]: [f32; 3]) {
    self.bb_min[0] = self.bb_min[0].min(x);
    self.bb_max[0] = self.bb_max[0].max(x);
    self.bb_min[1] = self.bb_min[1].min(y);
    self.bb_max[1] = self.bb_max[1].max(y);
    self.bb_min[2] = self.bb_min[2].min(z);
    self.bb_max[2] = self.bb_max[2].max(z);
  }
}

/// Errors that can occur when dealing with [`Obj`] objects.
#[derive(Debug)]
pub enum ObjError {
  /// Cannot parse the file.
  CannotParse(wavefront_obj::ParseError),

  /// Multi-object [`Obj`] are disallowed.
  ///
  /// The [`usize`] value is the number of objects this [`Obj`] is encoded with.
  MultiObject(usize),

  /// Multi-geometry [`Obj`] are not allowed.
  ///
  /// The [`usize`] value is the number of geometries this [`Obj`] is encoded with.
  MultiGeometry(usize),

  /// A vertex doesn’t have an associated normal.
  MissingNormal,

  /// Points, lines and polygons that are not a triangle are unsupported.
  NonTriangulated,
}

impl fmt::Display for ObjError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      ObjError::CannotParse(ref e) => write!(f, "cannot parse OBJ: {}", e),
      ObjError::MultiObject(nb) => {
        write!(f, "unsupported multi-object OBJ: {} objects present", nb)
      }
      ObjError::MultiGeometry(nb) => write!(
        f,
        "unsupported multi-geometries OBJ: {} geometries present",
        nb
      ),
      ObjError::MissingNormal => f.write_str("at least one vertex is missing its normal"),
      ObjError::NonTriangulated => f.write_str("non-triangulated data"),
    }
  }
}

/// Arbitrary object, with vertex positions, normals, and vertex indices.
#[derive(Clone, Debug)]
pub struct Obj {
  vertices: Vec<TessVertex3>,
  indices: Vec<TessIndex>,
  metadata: ObjMetadata,
}

impl Default for Obj {
  fn default() -> Self {
    Obj {
      vertices: Vec::new(),
      indices: Vec::new(),
      metadata: ObjMetadata::default(),
    }
  }
}

impl Obj {
  pub fn from_str(input: impl AsRef<str>) -> Result<Self, ObjError> {
    let obj_set = wavefront_obj::obj::parse(input).map_err(ObjError::CannotParse)?;
    let objects = obj_set.objects;

    if objects.len() != 1 {
      return Err(ObjError::MultiObject(objects.len()));
    }

    let object = objects.into_iter().next().unwrap();

    if object.geometry.len() != 1 {
      return Err(ObjError::MultiGeometry(object.geometry.len()));
    }

    let geometry = object.geometry.into_iter().next().unwrap();

    println!("loading {}", object.name);
    println!("{} vertices", object.vertices.len());
    println!("{} shapes", geometry.shapes.len());

    // build up vertices; for this to work, we remove duplicated vertices by putting them in a
    // map associating the vertex with its ID
    let mut vertex_cache: HashMap<wavefront_obj::obj::VTNIndex, TessIndex> = HashMap::new();
    let mut vertices: Vec<TessVertex3> = Vec::new();
    let mut indices: Vec<TessIndex> = Vec::new();

    // various metadata computed while traversing the object
    let mut metadata = ObjMetadata::new();

    for shape in geometry.shapes {
      if let wavefront_obj::obj::Primitive::Triangle(a, b, c) = shape.primitive {
        for key in &[a, b, c] {
          if let Some(vertex_index) = vertex_cache.get(key) {
            indices.push(*vertex_index);
          } else {
            let p = object.vertices[key.0];
            let n = object.normals[key.2.ok_or(ObjError::MissingNormal)?];
            let xyz = [p.x as f32, p.y as f32, p.z as f32];
            let pos = VPos3::new(xyz);
            let nor = VNor::new([n.x as f32, n.y as f32, n.z as f32]);
            let vertex = TessVertex3 { pos, nor };
            let vertex_index = vertices.len() as TessIndex;

            vertex_cache.insert(*key, vertex_index);
            vertices.push(vertex);
            indices.push(vertex_index);

            // update the metadata
            metadata.traverse_vertex(xyz);
          }
        }
      } else {
        return Err(ObjError::NonTriangulated);
      }
    }

    let obj = Obj {
      vertices,
      indices,
      metadata,
    };

    Ok(obj)
  }
}

impl Resource for Obj {
  type Source = Obj;
  type ResourceID = ResourceID<Obj>;
}
