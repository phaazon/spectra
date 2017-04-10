use luminance::tess::{Mode, Tess, TessVertices};

// A unit cube.
//
//     ×-----×
//    /|    /|
//   ×-+---× |
//   | ×---+-×
//   |/    |/
//   ×-----×
pub fn new_cube(with_normals: bool) -> Tess {
  if with_normals {
    let vertices = [
      // front face
      ([ 1., -1.,  1.], [ 0.,  0.,  1.]), // 0
      ([ 1.,  1.,  1.], [ 0.,  0.,  1.]),
      ([-1., -1.,  1.], [ 0.,  0.,  1.]),
      ([-1.,  1.,  1.], [ 0.,  0.,  1.]),
      // back face
      ([ 1., -1., -1.], [ 0.,  0., -1.]), // 4
      ([ 1.,  1., -1.], [ 0.,  0., -1.]),
      ([-1., -1., -1.], [ 0.,  0., -1.]),
      ([-1.,  1., -1.], [ 0.,  0., -1.]),
      // left face
      ([-1., -1.,  1.], [-1.,  0.,  0.]), // 8
      ([-1.,  1.,  1.], [-1.,  0.,  0.]),
      ([-1., -1., -1.], [-1.,  0.,  0.]),
      ([-1.,  1., -1.], [-1.,  0.,  0.]),
      // right face
      ([ 1., -1., -1.], [ 1.,  0.,  0.]), // 12
      ([ 1.,  1., -1.], [ 1.,  0.,  0.]),
      ([ 1., -1.,  1.], [ 1.,  0.,  0.]),
      ([ 1.,  1.,  1.], [ 1.,  0.,  0.]),
      // top face
      ([ 1.,  1.,  1.], [ 0.,  1., 0.]), // 16
      ([ 1.,  1., -1.], [ 0.,  1., 0.]),
      ([-1.,  1.,  1.], [ 0.,  1., 0.]),
      ([-1.,  1., -1.], [ 0.,  1., 0.]),
      // bottom face
      ([ 1., -1., -1.], [ 0., -1., 0.]), // 20
      ([ 1., -1.,  1.], [ 0., -1., 0.]),
      ([-1., -1., -1.], [ 0., -1., 0.]),
      ([-1., -1.,  1.], [ 0., -1., 0.]),
    ];

    let indices: &[u32] = &[
      0, 1, 2, 2, 1, 3, // front face
      4, 5, 6, 6, 5, 7, // back face
      8, 9, 10, 10, 9, 11, // left face
      12, 13, 14, 14, 13, 15, // right face
      16, 17, 18, 18, 17, 19, // top face
      20, 21, 22, 22, 21, 23, // bottom face
    ];

    Tess::new(Mode::Triangle, TessVertices::Fill(&vertices), indices)
  } else {
    let vertices = [
      [ 1., -1.,  1.],
      [ 1.,  1.,  1.],
      [-1., -1.,  1.],
      [-1.,  1.,  1.],
      [ 1., -1., -1.],
      [ 1.,  1., -1.],
      [-1., -1., -1.],
      [-1.,  1., -1.],
    ];

    let indices: &[u32] = &[
      0, 1, 2, 2, 1, 3, // front face
      1, 5, 3, 3, 5, 7, // top face
      2, 3, 6, 6, 3, 7, // right face
      4, 5, 0, 0, 5, 1, // left face
      4, 0, 6, 6, 0, 2, // bottom face
      4, 5, 6, 6, 5, 7, // back face
    ];

    Tess::new(Mode::Triangle, TessVertices::Fill(&vertices), indices)
  }
}
