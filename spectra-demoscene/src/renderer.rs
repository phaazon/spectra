use crate::{
  camera::FreeflyCamera,
  color::RGB,
  entity::Entity,
  light::DirLight,
  material::ColorMaterial,
  model::Model,
  obj::{Obj, ObjMetadata},
  resource::ResourceID,
  shaders,
  transform::Transform,
  vertex::{TessIndex, TessVertex3, TessVertex3Debug, VertexSemantics},
};
use cgmath::{Deg, Matrix4, Point3, Rad, Vector3};
use itertools::Itertools as _;
use luminance::{tess::TessVertexData, UniformInterface};
use luminance_front::{
  context::GraphicsContext,
  depth_test::DepthWrite,
  framebuffer::Framebuffer,
  pipeline::{PipelineError, PipelineState},
  render_state::RenderState,
  shader::{BuiltProgram, Program, ProgramError, ProgramInterface, Uniform, UniformInterface},
  tess::{Mode, Tess, TessError, TessView},
  tess_gate::TessGate,
  texture::Dim2,
  vertex::Semantics,
  Backend,
};
use spectra::renderer::DefaultFrame;
use std::{collections::HashMap, fmt, iter::once};

const INITIAL_FOVY: Deg<f32> = Deg(90.);
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100.;

/// Renderer settings.
#[non_exhaustive]
pub struct RendererSettings {
  /// Color of the bounding boxes.
  pub bounding_box_color: RGB,

  /// Thickness to use for objects without volume.
  pub no_volume_thickness: f32,

  /// Local axis scale multiplier.
  pub local_axis_scale_multiplier: f32,
}

impl Default for RendererSettings {
  fn default() -> Self {
    Self {
      bounding_box_color: [255, 0, 255],
      no_volume_thickness: 0.,
      local_axis_scale_multiplier: 0.5,
    }
  }
}

/// Error that can occur in the renderer.
#[derive(Debug)]
pub enum RendererError {
  /// The scene vertex shader couldn’t transpile.
  VertexShaderTranspile(fmt::Error, &'static str),

  /// The scene fragment shader couldn’t transpile.
  FragmentShaderTranspile(fmt::Error, &'static str),

  /// The scene shader couldn’t compile.
  ShaderCompile(ProgramError, &'static str),

  /// Cannot create a tessellation.
  ///
  /// The string represents the model that cannot be built.
  CannotBuildTess(TessError, &'static str),
}

impl fmt::Display for RendererError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      RendererError::VertexShaderTranspile(ref err, ref kind) => {
        write!(f, "cannot transpile {} vertex shader: {}", kind, err)
      }

      RendererError::FragmentShaderTranspile(ref err, ref kind) => {
        write!(f, "cannot transpile {} fragment shader: {}", kind, err)
      }

      RendererError::ShaderCompile(ref err, ref kind) => {
        write!(f, "cannot compile {} shader: {}", kind, err)
      }

      RendererError::CannotBuildTess(ref err, kind) => {
        write!(f, "cannot build {} tessellation: {}", kind, err)
      }
    }
  }
}

#[derive(Debug)]
struct OpaqueDispatchedProperties {
  transform: Transform,
  color_material: ColorMaterial,
}

impl OpaqueDispatchedProperties {
  fn new(transform: Transform, color_material: ColorMaterial) -> Self {
    Self {
      transform,
      color_material,
    }
  }
}

#[derive(Debug)]
struct DebugDispatchedProperties {
  transform: Transform,
}

impl DebugDispatchedProperties {
  fn new(transform: Transform) -> Self {
    Self { transform }
  }
}

/// Dispatched properties for bounding boxes.
///
/// Those properties are scales applied on each axis of the cube we use to render. The formula, on each `k` axis, is as
/// follows:
///
/// scale_k = (max_k - min_k) * 0.5
///
/// We divide by 2 because we are not using a unit cube, but twice one.
#[derive(Debug)]
struct DebugBoundingBoxDispatchedProperties {
  transform: Transform,
  scale: Vector3<f32>,
}

impl DebugBoundingBoxDispatchedProperties {
  fn new(transform: Transform, min: Vector3<f32>, max: Vector3<f32>) -> Self {
    let scale = (max - min) * 0.5;
    Self { transform, scale }
  }
}

/// The main renderer.
///
/// The main renderer is responsible in rendering the application on screen.
pub struct Renderer {
  settings: RendererSettings,
  debug: bool,
  /// Inversed of the resolution.
  ires: [f32; 2],
  pipeline_state: PipelineState,
  back_buffer: Framebuffer<Dim2, (), ()>,
  scene_shader: Program<VertexSemantics, (), SceneShaderUni>,
  obj_tess: HashMap<ResourceID<Obj>, (Tess<TessVertex3, TessIndex>, ObjMetadata)>,
  cube_tess: Tess<TessVertex3, TessIndex>,
  plane_tess: Tess<TessVertex3, ()>,
  opaque_model_rdr_st: RenderState,
  fullscreen_triangle_tess: Tess<()>,
  light_shader: Program<(), (), LightShaderUni>,
  sky_rdr_st: RenderState,

  // debug
  camera: FreeflyCamera,
  debug_bounding_box_tess: Tess<TessVertex3Debug, TessIndex>,
  debug_xyz_axis_shader: Program<(), (), DebugShaderUni>,
  debug_xyz_axis_tess: Tess<()>,
  debug_rdr_st: RenderState,
  debug_bounding_box_shader: Program<VertexSemantics, (), DebugBoundingBoxUni>,

  // ui
  ui_shader: Program<VertexSemantics, (), UIShaderUniform>,

  // entity render lists
  objs: Vec<(ResourceID<Obj>, OpaqueDispatchedProperties)>,
  cubes: Vec<OpaqueDispatchedProperties>,
  planes: Vec<OpaqueDispatchedProperties>,
  debug_axis: Vec<DebugDispatchedProperties>,
  debug_bounding_boxes: Vec<DebugBoundingBoxDispatchedProperties>,

  // local frame, provided to the user-side code to be filled
  frame: DefaultFrame<Entity, DirLight>,
}

impl Renderer {
  pub fn new(
    settings: RendererSettings,
    context: &mut impl GraphicsContext<Backend = Backend>,
    back_buffer: Framebuffer<Dim2, (), ()>,
    width: u32,
    height: u32,
  ) -> Result<Self, RendererError> {
    let pipeline_state = PipelineState::default();
    let scene_shader = Self::compile_vertex_fragment_shader(
      context,
      "scene",
      shaders::scene_vertex_shader(),
      shaders::scene_fragment_shader(),
    )?;
    let obj_tess = HashMap::new();
    let cube_tess = Self::create_cube_tess(context)?;
    let opaque_model_rdr_st = Self::opaque_model_render_state();
    let plane_tess = Self::create_finite_plane_tess(context)?;
    let (width, height) = (width as f32, height as f32);
    let aspect_ratio = width / height;
    let ires = [width.recip(), height.recip()];
    let fullscreen_triangle_tess = Self::create_fullscreen_triangle(context)?;
    let light_shader = Self::compile_vertex_fragment_shader(
      context,
      "light",
      shaders::dir_light_vertex_shader(),
      shaders::dir_light_fragment_shader(),
    )?;
    let sky_rdr_st = RenderState::default()
      .set_depth_write(DepthWrite::Off)
      .set_depth_test(None);

    // debug
    let camera = FreeflyCamera::new(aspect_ratio, INITIAL_FOVY, Z_NEAR, Z_FAR);
    let debug_bounding_box_tess = Self::create_debug_bounding_box_tess(context)?;
    let debug_xyz_axis_shader = Self::compile_vertex_fragment_shader(
      context,
      "debug xyz axis",
      shaders::debug_xyz_axis_vertex_shader(),
      shaders::debug_xyz_axis_fragment_shader(),
    )?;
    let (debug_xyz_axis_tess, debug_rdr_st) = Self::create_debug_xyz_axis_tess(context)?;
    let debug_bounding_box_shader = Self::compile_vertex_fragment_shader(
      context,
      "debug bounding box",
      shaders::debug_bounding_box_vertex_shader(),
      shaders::debug_bounding_box_fragment_shader(),
    )?;

    // ui
    let ui_shader = Self::compile_vertex_fragment_shader(
      context,
      "UI",
      shaders::ui_vertex_shader(),
      shaders::ui_fragment_shader(),
    )?;

    // dispatch queues
    let objs = Vec::new();
    let cubes = Vec::new();
    let planes = Vec::new();
    let debug_axis = Vec::new();
    let debug_bounding_boxes = Vec::new();

    // user-side frame
    let frame = DefaultFrame::new();

    Ok(Self {
      settings,
      debug: false,
      ires,
      pipeline_state,
      back_buffer,
      scene_shader,
      obj_tess,
      cube_tess,
      plane_tess,
      opaque_model_rdr_st,
      light_shader,
      fullscreen_triangle_tess,
      sky_rdr_st,
      camera,
      debug_bounding_box_tess,
      debug_xyz_axis_shader,
      debug_xyz_axis_tess,
      debug_rdr_st,
      debug_bounding_box_shader,
      ui_shader,
      objs,
      cubes,
      planes,
      debug_axis,
      debug_bounding_boxes,
      frame,
    })
  }

  /// Resize the framebuffer.
  pub fn resize(&mut self, back_buffer: Framebuffer<Dim2, (), ()>) {
    let [w, h] = back_buffer.size();
    let (w, h) = (w as f32, h as f32);
    let aspect_ratio = w / h;
    self.camera.set_aspect_ratio(aspect_ratio);
    self.back_buffer = back_buffer;
    self.ires = [w.recip(), h.recip()];
  }

  /// Accept new settings.
  pub fn accept_settings(&mut self, settings: RendererSettings) {
    self.settings = settings;
  }

  /// Toggle the debug mode on and off.
  pub fn toggle_debug(&mut self) {
    self.debug = !self.debug;
  }

  /// Compile a vertex + fragment shader combo.
  fn compile_vertex_fragment_shader<S, U>(
    context: &mut impl GraphicsContext<Backend = Backend>,
    name: &'static str,
    vertex_shader: shades::Shader,
    fragment_shader: shades::Shader,
  ) -> Result<Program<S, (), U>, RendererError>
  where
    S: Semantics,
    U: UniformInterface<Backend>,
  {
    // stages
    let vs = shades::writer::glsl::write_shader_to_str(vertex_shader)
      .map_err(|e| RendererError::VertexShaderTranspile(e, name))?;
    log::info!("created {} vertex shader:", name);
    for line in vs.lines() {
      log::debug!("  {}", line);
    }

    let fs = shades::writer::glsl::write_shader_to_str(fragment_shader)
      .map_err(|e| RendererError::FragmentShaderTranspile(e, name))?;
    log::info!("created {} fragment shader:", name);
    for line in fs.lines() {
      log::debug!("  {}", line);
    }

    // program
    let BuiltProgram { program, warnings } = context
      .new_shader_program::<S, (), U>()
      .from_strings(&vs, None, None, &fs)
      .map_err(|e| RendererError::ShaderCompile(e, name))?;
    log::info!("created {} shader", name);

    for warn in warnings {
      log::warn!("{} shader warning: {}", name, warn);
    }

    Ok(program)
  }

  /// Render state used for opaque models.
  fn opaque_model_render_state() -> RenderState {
    RenderState::default()
  }

  /// Create tessellation to hold the cube model.
  fn create_cube_tess(
    context: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<Tess<TessVertex3, TessIndex>, RendererError> {
    // the cube will have edges of length = 1; vertices are expressed with a fan mode
    let vertices = [
      // front face
      TessVertex3::new([-1., 1., 1.].into(), [0., 0., 1.].into()),
      TessVertex3::new([-1., -1., 1.].into(), [0., 0., 1.].into()),
      TessVertex3::new([1., -1., 1.].into(), [0., 0., 1.].into()),
      TessVertex3::new([1., 1., 1.].into(), [0., 0., 1.].into()),
      // back face
      TessVertex3::new([1., 1., -1.].into(), [0., 0., -1.].into()),
      TessVertex3::new([1., -1., -1.].into(), [0., 0., -1.].into()),
      TessVertex3::new([-1., -1., -1.].into(), [0., 0., -1.].into()),
      TessVertex3::new([-1., 1., -1.].into(), [0., 0., -1.].into()),
      // top face
      TessVertex3::new([-1., 1., -1.].into(), [0., 1., 0.].into()),
      TessVertex3::new([-1., 1., 1.].into(), [0., 1., 0.].into()),
      TessVertex3::new([1., 1., 1.].into(), [0., 1., 0.].into()),
      TessVertex3::new([1., 1., -1.].into(), [0., 1., 0.].into()),
      // bottom face
      TessVertex3::new([-1., -1., 1.].into(), [0., -1., 0.].into()),
      TessVertex3::new([-1., -1., -1.].into(), [0., -1., 0.].into()),
      TessVertex3::new([1., -1., -1.].into(), [0., -1., 0.].into()),
      TessVertex3::new([1., -1., 1.].into(), [0., -1., 0.].into()),
      // left face
      TessVertex3::new([-1., 1., -1.].into(), [1., 0., 0.].into()),
      TessVertex3::new([-1., -1., -1.].into(), [1., 0., 0.].into()),
      TessVertex3::new([-1., -1., 1.].into(), [1., 0., 0.].into()),
      TessVertex3::new([-1., 1., 1.].into(), [1., 0., 0.].into()),
      // right face
      TessVertex3::new([1., 1., 1.].into(), [-1., 0., 0.].into()),
      TessVertex3::new([1., -1., 1.].into(), [-1., 0., 0.].into()),
      TessVertex3::new([1., -1., -1.].into(), [-1., 0., 0.].into()),
      TessVertex3::new([1., 1., -1.].into(), [-1., 0., 0.].into()),
    ];
    let indices = (0..24)
      .into_iter()
      .chunks(4)
      .into_iter()
      .flat_map(|chunk| chunk.chain(once(u32::MAX)))
      .collect::<Vec<_>>();

    let tess = context
      .new_tess()
      .set_vertices(vertices)
      .set_indices(indices)
      .set_mode(Mode::TriangleFan)
      .set_primitive_restart_index(u32::MAX)
      .build()
      .map_err(|e| RendererError::CannotBuildTess(e, "cube"))?;
    log::info!("created cube model tessellation");

    Ok(tess)
  }

  /// Create a fullscreen triangle.
  fn create_fullscreen_triangle(
    context: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<Tess<()>, RendererError> {
    context
      .new_tess()
      .set_mode(Mode::Triangle)
      .set_render_vertex_nb(3)
      .build()
      .map_err(|e| RendererError::CannotBuildTess(e, "fullscreen triangle"))
  }

  /// Create tessellation to hold the debug cube model.
  fn create_debug_bounding_box_tess(
    context: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<Tess<TessVertex3Debug, TessIndex>, RendererError> {
    let vertices = [
      // top plane
      TessVertex3Debug::new([-1., 1., 1.].into()),
      TessVertex3Debug::new([1., 1., 1.].into()),
      TessVertex3Debug::new([1., 1., -1.].into()),
      TessVertex3Debug::new([-1., 1., -1.].into()),
      // bottom plane
      TessVertex3Debug::new([-1., -1., 1.].into()),
      TessVertex3Debug::new([1., -1., 1.].into()),
      TessVertex3Debug::new([1., -1., -1.].into()),
      TessVertex3Debug::new([-1., -1., -1.].into()),
    ];

    let indices = [
      // top plane
      0,
      1,
      2,
      3,
      0,
      u32::MAX,
      // bottom plane
      4,
      5,
      6,
      7,
      4,
      u32::MAX,
      // top-bottom joins
      // left near,
      0,
      4,
      u32::MAX,
      // right near
      1,
      5,
      u32::MAX,
      // right far
      2,
      6,
      u32::MAX,
      // left far
      3,
      7,
    ];

    let tess = context
      .new_tess()
      .set_vertices(vertices)
      .set_indices(indices)
      .set_mode(Mode::LineStrip)
      .set_primitive_restart_index(u32::MAX)
      .build()
      .map_err(|e| RendererError::CannotBuildTess(e, "debug cube"))?;

    Ok(tess)
  }

  fn create_finite_plane_tess(
    context: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<Tess<TessVertex3, ()>, RendererError> {
    let vertices = [
      TessVertex3::new([-1., -1., 0.].into(), [0., 0., 1.].into()),
      TessVertex3::new([-1., 1., 0.].into(), [0., 0., 1.].into()),
      TessVertex3::new([1., 1., 0.].into(), [0., 0., 1.].into()),
      TessVertex3::new([1., -1., 0.].into(), [0., 0., 1.].into()),
    ];
    let tess = context
      .new_tess()
      .set_vertices(vertices)
      .set_mode(Mode::TriangleFan)
      .build()
      .map_err(|e| RendererError::CannotBuildTess(e, "finite plane"))?;
    log::info!("created plane model tessellation");

    Ok(tess)
  }

  fn create_debug_xyz_axis_tess(
    context: &mut impl GraphicsContext<Backend = Backend>,
  ) -> Result<(Tess<()>, RenderState), RendererError> {
    let tess = context
      .new_tess()
      .set_render_vertex_nb(6)
      .set_mode(Mode::Line)
      .build()
      .map_err(|e| RendererError::CannotBuildTess(e, "debug XYZ axis"))?;
    log::info!("created debug tessellation");

    let rdr_st = RenderState::default().set_depth_test(None);

    Ok((tess, rdr_st))
  }

  pub fn set_camera_field_of_view(&mut self, fovy: impl Into<Rad<f32>>) {
    self.camera.set_field_of_view(fovy);
  }

  pub fn set_camera_z_near(&mut self, z_near: f32) {
    self.camera.set_z_near(z_near);
  }

  pub fn set_camera_z_far(&mut self, z_far: f32) {
    self.camera.set_z_far(z_far);
  }

  pub fn move_camera(&mut self, v: Vector3<f32>) {
    self.camera.move_by(-v);
  }

  pub fn orient_camera(&mut self, x_theta: impl Into<Rad<f32>>, y_theta: impl Into<Rad<f32>>) {
    self.camera.orient(x_theta, y_theta);
  }

  /// Register an [`Obj`] into the GPU.
  pub fn register_obj(
    &mut self,
    context: &mut impl GraphicsContext<Backend = Backend>,
    model: Obj,
    id: ResourceID<Obj>,
  ) -> Result<(), RendererError> {
    let (tess, metadata) = model
      .to_tess(context)
      .map_err(|e| RendererError::CannotBuildTess(e, "obj"))?;

    self.obj_tess.insert(id, (tess, metadata));

    Ok(())
  }

  /// Dispatch incoming entities into their respective frame buckets.
  fn dispatch_entities(&mut self) {
    // clear the queues
    self.objs.clear();
    self.cubes.clear();
    self.planes.clear();
    self.debug_axis.clear();
    self.debug_bounding_boxes.clear();

    for entity in &self.render.entities {
      let transform = entity.transform;

      match entity.model {
        Model::OBJ {
          id,
          ref color_material,
        } => {
          self.objs.push((
            id,
            OpaqueDispatchedProperties::new(transform, color_material.clone()),
          ));

          // in debug mode, inject a bounding box to visualize around the object
          if self.debug {
            let local_axis_scale_multiplier =
              self.settings.debug.help_object.local_axis_scale_multiplier;

            // TODO: remove the unwrap
            let metadata = &self.obj_tess.get(&id).unwrap().1;
            self
              .debug_bounding_boxes
              .push(DebugBoundingBoxDispatchedProperties::new(
                transform,
                metadata.bb_min.into(),
                metadata.bb_max.into(),
              ));

            self.debug_axis.push(DebugDispatchedProperties::new(
              transform.concat(&Matrix4::from_scale(local_axis_scale_multiplier)),
            ));
          }
        }

        Model::Cube { ref color_material } => {
          self.cubes.push(OpaqueDispatchedProperties::new(
            transform,
            color_material.clone(),
          ));

          // in debug mode, inject a bounding box to visualize around the object
          if self.debug {
            self
              .debug_bounding_boxes
              .push(DebugBoundingBoxDispatchedProperties::new(
                transform,
                Vector3::new(-1., -1., -1.),
                Vector3::new(1., 1., 1.),
              ));
          }
        }

        Model::FinitePlane { ref color_material } => {
          self.planes.push(OpaqueDispatchedProperties::new(
            transform,
            color_material.clone(),
          ));

          // in debug mode, inject a bounding box to visualize around the object
          if self.debug {
            let thickness = self.settings.debug.help_object.no_volume_thickness;

            self
              .debug_bounding_boxes
              .push(DebugBoundingBoxDispatchedProperties::new(
                transform,
                Vector3::new(-1., -1., -thickness),
                Vector3::new(1., 1., thickness),
              ));
          }
        }
      }
    }

    if self.render.debug_show_origin {
      self
        .debug_axis
        .push(DebugDispatchedProperties::new(Transform::one()));
    }
  }

  fn render_opaque_objects<'a, V, I, W, S>(
    uni: &SceneShaderUni,
    iface: &mut ProgramInterface,
    projview: &Matrix4<f32>,
    eye: &Point3<f32>,
    light_dir: &Vector3<f32>,
    dispatched_properties: &OpaqueDispatchedProperties,
    tess_gate: &mut TessGate<'a>,
    tess: impl Into<TessView<'a, V, I, W, S>>,
  ) -> Result<(), PipelineError>
  where
    Backend: luminance::backend::tess::Tess<V, I, W, S>
      + luminance::backend::tess_gate::TessGate<V, I, W, S>,
    V: 'a + TessVertexData<S>,
    I: 'a + luminance::tess::TessIndex,
    W: 'a + TessVertexData<S>,
    S: 'a,
  {
    let tess = tess.into();

    // lighting; transform light_dir into the model’s space coordinate
    let inversed_transform = dispatched_properties
      .transform
      .inverse_transform()
      .expect("opaque object transform inverse matrix");
    let model_light_dir = inversed_transform
      .transform_vector((*light_dir).into())
      .normalize();
    let model_eye = inversed_transform.transform_point(*eye);

    iface.set(&uni.light_dir, model_light_dir.into());
    iface.set(&uni.eye, model_eye.into());

    // if we have a material, render with it
    iface.set(
      &uni.material_ambient,
      dispatched_properties.color_material.ambient.into(),
    );
    iface.set(
      &uni.material_diffuse,
      dispatched_properties.color_material.diffuse.into(),
    );
    iface.set(
      &uni.material_specular,
      dispatched_properties.color_material.specular.into(),
    );
    iface.set(
      &uni.material_shininess,
      dispatched_properties.color_material.shininess,
    );

    // camera
    let projview_model = projview.concat(&dispatched_properties.transform);
    iface.set(&uni.projview_model, projview_model.into());

    tess_gate.render(tess)
  }

  pub fn animate(
    &mut self,
    context: &mut impl GraphicsContext<Backend = Backend>,
    store: &Store,
    animation: &mut impl RenderLogic,
  ) {
    self.render.prepare();

    let t = self.current_time();
    animation.animate(store, t, &mut self.render);

    // dispatch the entities
    self.dispatch_entities();

    // create the graphics pipeline
    let settings = &self.settings;
    let scene_shader = &mut self.scene_shader;
    let opaque_model_rdr_st = &self.opaque_model_rdr_st;
    let obj_tess = &self.obj_tess;
    let cube_tess = &self.cube_tess;
    let plane_tess = &self.plane_tess;
    let projview = self.camera.projection_view();
    let light_shader = &mut self.light_shader;
    let fullscreen_triangle = &self.fullscreen_triangle_tess;
    let sky_rdr_st = &self.sky_rdr_st;
    let ires = self.ires;

    // debug
    let debug_rdr_st = &self.debug_rdr_st;
    let debug_shader = &mut self.debug_xyz_axis_shader;
    let debug_bounding_box_shader = &mut self.debug_bounding_box_shader;
    let debug_xyz_axis_tess = &self.debug_xyz_axis_tess;
    let debug_bounding_box_tess = &self.debug_bounding_box_tess;

    // UI
    let ui_shader = &mut self.ui_shader;

    // dispatch queues
    let objs = &self.objs;
    let cubes = &self.cubes;
    let planes = &self.planes;
    let debug_axis = &self.debug_axis;
    let debug_bounding_boxes = &self.debug_bounding_boxes;

    let camera = &self.camera;

    let render = &self.render;

    let render = context
      .new_pipeline_gate()
      .pipeline(
        &self.back_buffer,
        &self.pipeline_state,
        |_, mut shd_gate| {
          // UI
          shd_gate.shade(ui_shader, |mut iface, uni, mut _rdr_gate| {
            iface.set(&uni.ires, ires);
            iface.set(&uni.aspect_ratio, camera.aspect_ratio());

            // TODO: render something <12-04-21, Dimitri Sabadie> //
            Ok(())
          })?;

          // then, render distant / sky things
          // FIXME: check first whether we have things to render, otherwise we are wasting GPU for nothing here
          shd_gate.shade(light_shader, |mut iface, uni, mut rdr_gate| {
            let light_dir = camera.view().transform_vector(render.light_scene.dir);
            let cam_depth = (camera.field_of_view().0 * 0.5).tan().recip();

            iface.set(&uni.aspect_ratio, camera.aspect_ratio());
            iface.set(&uni.light_dir, light_dir.normalize().into());
            iface.set(&uni.light_color, render.light_scene.color.into());
            iface.set(&uni.light_scattering, render.light_scene.scattering);
            iface.set(&uni.light_power, render.light_scene.power);
            iface.set(&uni.cam_depth, cam_depth);

            rdr_gate.render(sky_rdr_st, |mut tess_gate| {
              tess_gate.render(fullscreen_triangle)
            })
          })?;

          // render opaque objects second
          shd_gate.shade(scene_shader, |mut iface, uni, mut rdr_gate| {
            iface.set(&uni.t, t);
            iface.set(&uni.light_color, render.light_scene.color.into());
            iface.set(&uni.light_power, render.light_scene.power);

            rdr_gate.render(opaque_model_rdr_st, |mut tess_gate| {
              // render cubes
              for dispatched_properties in cubes {
                Self::render_opaque_objects(
                  uni,
                  &mut iface,
                  &projview,
                  camera.position(),
                  &render.light_scene.dir,
                  dispatched_properties,
                  &mut tess_gate,
                  cube_tess,
                )?;
              }

              // render the finite planes
              for dispatched_properties in planes {
                Self::render_opaque_objects(
                  uni,
                  &mut iface,
                  &projview,
                  camera.position(),
                  &render.light_scene.dir,
                  dispatched_properties,
                  &mut tess_gate,
                  plane_tess,
                )?;
              }

              // render all the user-defined objects
              for (model_handle, dispatched_properties) in objs {
                if let Some((ref tess, _)) = obj_tess.get(&model_handle) {
                  Self::render_opaque_objects(
                    uni,
                    &mut iface,
                    &projview,
                    camera.position(),
                    &render.light_scene.dir,
                    dispatched_properties,
                    &mut tess_gate,
                    tess,
                  )?;
                }
              }

              Ok(())
            })
          })?;

          // render the debug axis
          shd_gate.shade(debug_shader, |mut iface, uni, mut rdr_gate| {
            for dispatched_props in debug_axis {
              let projview_model = projview.concat(&dispatched_props.transform);
              iface.set(&uni.projview_model, projview_model.into());

              rdr_gate.render(debug_rdr_st, |mut tess_gate| {
                tess_gate.render(debug_xyz_axis_tess)
              })?;
            }

            Ok(())
          })?;

          // render bounding boxes
          shd_gate.shade(debug_bounding_box_shader, |mut iface, uni, mut rdr_gate| {
            for dispatched_bb in debug_bounding_boxes {
              let projview_model = projview.concat(&dispatched_bb.transform);
              iface.set(&uni.projview_model, projview_model.into());
              iface.set(&uni.scale, dispatched_bb.scale.into());
              iface.set(&uni.color, settings.debug.help_object.bounding_box_color);

              rdr_gate.render(debug_rdr_st, |mut tess_gate| {
                tess_gate.render(debug_bounding_box_tess)
              })?;
            }

            Ok(())
          })
        },
      )
      .assume()
      .into_result();

    if let Err(err) = render {
      log::error!("cannot render frame: {}", err);
    }
  }
}

#[derive(Debug, UniformInterface)]
pub struct SceneShaderUni {
  #[uniform(unbound)]
  t: Uniform<f32>,
  projview_model: Uniform<[[f32; 4]; 4]>,
  light_dir: Uniform<[f32; 3]>,
  light_color: Uniform<[f32; 3]>,
  light_power: Uniform<f32>,

  eye: Uniform<[f32; 3]>,

  // color material things
  material_ambient: Uniform<[f32; 3]>,
  material_diffuse: Uniform<[f32; 3]>,
  material_specular: Uniform<[f32; 3]>,
  material_shininess: Uniform<f32>,
}

#[derive(Debug, UniformInterface)]
pub struct DebugShaderUni {
  projview_model: Uniform<[[f32; 4]; 4]>,
}

#[derive(Debug, UniformInterface)]
pub struct DebugBoundingBoxUni {
  projview_model: Uniform<[[f32; 4]; 4]>,
  scale: Uniform<[f32; 3]>,
  color: Uniform<[f32; 3]>,
}

#[derive(Debug, UniformInterface)]
pub struct LightShaderUni {
  aspect_ratio: Uniform<f32>,
  light_dir: Uniform<[f32; 3]>,
  light_color: Uniform<[f32; 3]>,
  light_scattering: Uniform<f32>,
  light_power: Uniform<f32>,
  cam_depth: Uniform<f32>,
}

#[derive(Debug, UniformInterface)]
pub struct UIShaderUniform {
  ires: Uniform<[f32; 2]>,
  aspect_ratio: Uniform<f32>,
}
