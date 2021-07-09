use luminance::{Semantics, Vertex};

#[derive(Clone, Copy, Debug, Semantics)]
pub enum VertexSemantics {
  #[sem(name = "pos", repr = "[f32; 2]", wrapper = "VPos2")]
  Position2,
  #[sem(name = "pos", repr = "[f32; 3]", wrapper = "VPos3")]
  Position3,
  #[sem(name = "nor", repr = "[f32; 3]", wrapper = "VNor")]
  Normal,
  #[sem(name = "col", repr = "[u8; 3]", wrapper = "VCol")]
  RGB,
}

/// Vertex type used for most objects. They have a position and a normal.
#[derive(Clone, Copy, Debug, Vertex)]
#[vertex(sem = "VertexSemantics")]
pub struct TessVertex3 {
  pub pos: VPos3,
  pub nor: VNor,
}

/// Vertex type for debug / helper objects. The only attribute they have is their positions in 3D.
#[derive(Clone, Copy, Debug, Vertex)]
#[vertex(sem = "VertexSemantics")]
pub struct TessVertex3Debug {
  pub pos: VPos3,
}

pub type TessIndex = u32;
