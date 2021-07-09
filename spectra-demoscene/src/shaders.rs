use shades::{
  inputs, lit, outputs, uniforms, vec2, vec3, vec4, Bounded as _, Exponential as _, Expr,
  Geometry as _, HasX as _, HasY as _, Scope, Shader, ShaderBuilder, M44, V2, V3, V4,
};

pub fn scene_vertex_shader() -> Shader {
  ShaderBuilder::new_vertex_shader(|mut s, vertex| {
    inputs!(s, pos: V3<f32>, nor: V3<f32>);
    outputs!(s, v_nor: V3<f32>, v_pos: V3<f32>);
    uniforms!(s, projview_model: M44);

    s.main_fun(|s: &mut Scope<()>| {
      s.set(vertex.position, projview_model * vec4!(pos, 1.));
      s.set(v_nor, nor);
      s.set(v_pos, pos);
    })
  })
}

pub fn scene_fragment_shader() -> Shader {
  ShaderBuilder::new_fragment_shader(|mut s, _fragment| {
    inputs!(s, v_nor: V3<f32>, v_pos: V3<f32>);
    outputs!(s, f_color: V4<f32>);
    uniforms!(
      s,
      light_dir: V3<f32>,
      light_color: V3<f32>,
      light_power: f32,
      eye: V3<f32>,
      material_ambient: V3<f32>,
      material_diffuse: V3<f32>,
      material_specular: V3<f32>,
      material_shininess: f32
    );

    s.main_fun(|s: &mut Scope<()>| {
      // ambient color
      let ambient_color = material_ambient;

      // diffuse color
      let diffuse_color = material_diffuse;
      let kd = s.var(light_dir.dot(-&v_nor).max(0.));

      // specular color
      let specular_color = material_specular;
      let specular_shininess = material_shininess;
      let eye_dir = (v_pos - eye).normalize();
      let ks = s.var(
        light_dir
          .dot(-eye_dir.reflect(v_nor).normalize())
          .max(0.)
          .pow(specular_shininess),
      );

      let color = s.var(ambient_color + diffuse_color * kd + specular_color * ks) * light_power;
      let corrected = (color * light_color).pow(lit!(1., 1., 1.) / 2.2);

      s.set(f_color, vec4!(corrected, 1.));
    })
  })
}

pub fn debug_xyz_axis_vertex_shader() -> Shader {
  ShaderBuilder::new_vertex_shader(|mut s, vertex| {
    outputs!(s, v_col: V3<f32>);
    uniforms!(s, projview_model: M44);

    let positions = s.constant([
      // X axis
      vec3!(0., 0., 0.),
      vec3!(1., 0., 0.),
      // Y axis
      vec3!(0., 0., 0.),
      vec3!(0., 1., 0.),
      // Z axis
      vec3!(0., 0., 0.),
      vec3!(0., 0., 1.),
    ]);
    let colors = s.constant([
      // X axis
      vec3!(1., 0., 0.),
      vec3!(1., 0., 0.),
      // Y axis
      vec3!(0., 1., 0.),
      vec3!(0., 1., 0.),
      // Z axis
      vec3!(0., 0., 1.),
      vec3!(0., 0., 1.),
    ]);

    s.main_fun(|s: &mut Scope<()>| {
      let pos = positions.at(&vertex.vertex_id);
      s.set(vertex.position, projview_model * vec4!(pos, 1.));
      s.set(v_col, colors.at(vertex.vertex_id));
    })
  })
}

pub fn debug_xyz_axis_fragment_shader() -> Shader {
  ShaderBuilder::new_fragment_shader(|mut s, _fragment| {
    inputs!(s, v_col: V3<f32>);
    outputs!(s, f_color: V4<f32>);

    s.main_fun(|s: &mut Scope<()>| {
      s.set(f_color, vec4!(v_col, 1.));
    })
  })
}

pub fn debug_bounding_box_vertex_shader() -> Shader {
  ShaderBuilder::new_vertex_shader(|mut s, vertex| {
    inputs!(s, pos: V3<f32>);
    uniforms!(s, projview_model: M44, scale: V3<f32>);

    s.main_fun(|s: &mut Scope<()>| {
      s.set(vertex.position, projview_model * vec4!(pos * scale, 1.));
    })
  })
}

pub fn debug_bounding_box_fragment_shader() -> Shader {
  ShaderBuilder::new_fragment_shader(|mut s, _| {
    outputs!(s, f_color: V4<f32>);
    uniforms!(s, color: V3<f32>);

    s.main_fun(|s: &mut Scope<()>| {
      s.set(f_color, vec4!(color, 1.));
    })
  })
}

//
//         (-1, 3)
//           x
//           |
//           |
//        (-1, 1)              (1, 1)
//           +-------------------+
//           |                   |
//           |                   |
//           |                   |
//           +-------------------+-------------------x
//       (-1, -1)             (1, -1)             (3, -1)
//
pub fn fullscreen_tri() -> Expr<[V2<f32>; 3]> {
  lit!([lit!(-1., 3.), lit!(-1., -1.), lit!(3., -1.)])
}

pub fn dir_light_vertex_shader() -> Shader {
  ShaderBuilder::new_vertex_shader(|mut s, vertex| {
    outputs!(s, v_pos: V2<f32>);
    uniforms!(s, aspect_ratio: f32);

    let triangle = s.constant(fullscreen_tri());

    s.main_fun(|s: &mut Scope<()>| {
      let p = s.var(triangle.at(vertex.vertex_id));
      s.set(vertex.position, vec4!(p.x(), p.y() * aspect_ratio, 0., 1.));
      s.set(v_pos, p);
    })
  })
}

//
//                 (0, 0, 0)
//             |-------·-------| screen
//              \      |      /
//               \     |     /
//                \    |    /
//                 \   |   /
//                  \  |  /
//                   \ |α/
//                    \|/
//                     · camera of fovy = φ, α = φ/2
//                 (0, 0, d)
//
//  to compute d, highschool:
//
//    tan(α) = 1 / cam_depth
//    cam_depth = 1 / tan(α)
//    cam_depth = 1 / tan(φ / 2)
//
pub fn dir_light_fragment_shader() -> Shader {
  ShaderBuilder::new_fragment_shader(|mut s, _| {
    inputs!(s, v_pos: V2<f32>);
    outputs!(s, f_color: V4<f32>);
    uniforms!(
      s,
      light_dir: V3<f32>,
      light_color: V3<f32>,
      light_scattering: f32,
      light_power: f32,
      cam_depth: f32
    );

    s.main_fun(|s: &mut Scope<()>| {
      let ray = vec3!(v_pos, -cam_depth).normalize();
      let cos_angle = s.var(ray.dot(-light_dir)).max(0.).pow(light_scattering) * light_power;
      let color = (light_color * cos_angle).pow(lit!(1., 1., 1.) / 2.2);

      s.set(&f_color, vec4!(color, 1.));
    })
  })
}

// A handy function to go from window space ([0; width -1]) to normalized screen space ([-1; 1]).
//
// The first argument `p` is the point to transform and `ires` is the inverse of the framebuffer resolution.
fn win_to_ss(p: &Expr<V2<f32>>, ires: &Expr<V2<f32>>, aspect_ratio: &Expr<f32>) -> Expr<V2<f32>> {
  (p * ires * 2. - 1.) * vec2!(1., *aspect_ratio)
}

pub fn ui_vertex_shader() -> Shader {
  ShaderBuilder::new_vertex_shader(|mut s, vertex| {
    inputs!(s, pos: V2<f32>, dim: V2<f32>, col: V4<f32>);
    outputs!(s, v_col: V4<f32>);
    uniforms!(s, ires: V2<f32>, aspect_ratio: f32);

    s.main_fun(|s: &mut Scope<()>| {
      // ss_pos is the screen-space position of the top-left corner of the rectangle; ss_dim is the screen-space
      // dimension of the rectangle, for which the height field is negated (to go down);
      let ss_pos = s.var(win_to_ss(&pos, &ires, &aspect_ratio));
      let ss_dim = s.var(win_to_ss(&dim, &ires, &aspect_ratio)) * vec2!(1., -1.);
      let rect = s.var([
        ss_pos.to_expr(),
        &ss_pos + vec2!(0., ss_dim.y()),
        &ss_pos + &ss_dim,
        ss_pos + vec2!(ss_dim.x(), 0.),
      ]);

      let p = s.var(rect.at(vertex.vertex_id));

      s.set(vertex.position, vec4!(p, 0., 1.));
      s.set(v_col, col);
    })
  })
}

pub fn ui_fragment_shader() -> Shader {
  ShaderBuilder::new_fragment_shader(|mut s, _| {
    inputs!(s, v_col: V4<f32>);
    outputs!(s, f_color: V4<f32>);

    s.main_fun(|s: &mut Scope<()>| {
      s.set(f_color, v_col);
    })
  })
}
