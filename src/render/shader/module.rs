//! Shader module.
//!
//! Shader functions and declarations can be grouped in so-called *modules*. Modules structure is
//! inherently tied to the filesystem’s tree.
//!
//! You’re not supposed to use modules at the Rust level, even though you can. You actually want to
//! write modules that will be used by shader programs using the SPSL language.
//!
//! # SPSL
//!
//! Spectra Shading Language is a superset of [GLSL](https://en.wikipedia.org/wiki/OpenGL_Shading_Language)
//! with extra candies, such as:
//!
//! - module imports/exports;
//! - interface, uniforms, blocks, structs, etc. deduplication
//! - functional programming style
//!
//! ## Define once, use everywhere
//!
//! The idea is that you can refactor the code you use at several places into *modules* – in the
//! same way you do in Rust, and then `import` those modules in other ones.
//!
//! This is achieved with the `from foo.bar.zoo import (yyyy, zzzz)` pattern. You typically want to put
//! that line at the top of your module – you can put several. This will import the `yyyy` and
//! `zzzz` symbols from the `foo.bar.zoo` module. The `(*)` form is called an import list and must
//! contain something.
//!
//! > Note on paths: because of the bijective relation between modules and files, if you import the
//! > `foo.bar.zoo` module, the file `foo/bar/zoo.spsl` must be reachable.
//!
//! > Note on import lists: currently, import lists are just informative. By default, all symbols
//! > are imported. Future plans will restrict them to the one only defined in the import lists.
//!
//! ## Pipeline modules
//!
//! In SPSL, there’s no such thing such as a *stage*. You cannot declare a *vertex shader*, a
//! *geometry shader*, a *fragment shader* or any *tessellation shaders*. Instead, you write
//! pipelines directly.
//!
//! A pipeline is just a special module that contains special functions. Up to now, you can find
//! three functions:
//!
//! | Function name     | Mandatory? | Role                                                              |
//! | -------------     | ---------- | ----                                                              |
//! | `map_vertex`      | yes        | Called on each vertex in the pipeline’s stream                    |
//! | `concat_map_prim` | no         | Called on each primitive generated via the `map_vertex` function  |
//! | `map_fragment`    | yes        | Called on each rasterized fragment                                |
//!
//! ### `map_vertex`
//!
//! This mandatory function must be defined and will be called on each vertex in the input stream.
//! It takes a variable number of arguments and its return type must be provided. Both the arguments
//! and return types form a *contract* that binds the function to the input and output stream. The
//! order of the arguments matters, as it must be the same order as in your tessellation’s buffers.
//!
//! For instance, if you want to process a stream of vertices which have a 3D-floating position and
//! a 4D-floating color and return only the color, you’d something like this:
//!
//! ```glsl
//! struct Vertex {
//!   vec4 spsl_Position; // this is mandatory as it will be fetched by the pipeline
//!   vec4 color;
//! };
//!
//! Vertex map_vertex(vec3 position, vec4 color) {
//!   return Vertex(vec4(position, 1.), color);
//! }
//! ```
//!
//! If at some time you come to the realization that you also need the position information in the
//! result, you just have to change the above code to:
//!
//! ```glsl
//! struct Vertex {
//!   vec4 spsl_Position; // this is mandatory as it will be fetched by the pipeline
//!   vec3 position;
//!   vec4 color;
//! };
//!
//! Vertex map_vertex(vec3 position, vec4 color) {
//!   return Vertex(vec4(position, 1.), position, color);
//! }
//! ```
//!
//! > Note on the return type: the name of this type is completely up to you. Nothing is enforced,
//! > use the type name you think is the best. `Vertex` is a de facto name because it seems natural
//! > to use it, but if you dislike such a name, feel free to use another.
//!
//! ### `concat_map_prim`
//!
//! This optional function takes an array of vertices which type is the same as `map_vertex`’
//! result’s type and outputs a stream of primitives:
//!
//! ```glsl
//! layout (triangles_strip, max_vertices = 3) struct Prim {
//!   // TODO
//! };
//!
//! void concat_map_prim(Vertex[3] vertices) {
//!   
//! }
//! ```

use glsl::writer;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use render::shader::lang::parser;
use render::shader::lang::syntax;
use sys::resource::{CacheKey, Load, LoadError, LoadResult, Store, StoreKey};

/// Key to use to get a `Module`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ModuleKey(String);

impl ModuleKey {
  /// Create a new module key. The string must contain the module in the form:
  ///
  ///     foo.bar.zoo
  pub fn new(key: &str) -> Self {
    ModuleKey(key.to_owned())
  }
}

impl CacheKey for ModuleKey {
  type Target = Module;
}

impl StoreKey for ModuleKey {
  fn key_to_path(&self) -> PathBuf {
    PathBuf::from(self.0.replace(".", "/") + ".spsl")
  }
}


impl Load for Module {
  type Key = ModuleKey;

  fn load(key: &Self::Key, _: &mut Store) -> Result<LoadResult<Self>, LoadError> {
    let path = key.key_to_path();

    let mut fh = File::open(&path).map_err(|_| LoadError::FileNotFound(path.into()))?;
    let mut src = String::new();
    let _ = fh.read_to_string(&mut src);

    match parser::parse_str(&src[..], parser::module) {
      parser::ParseResult::Ok(module) => {
        Ok(Module(module).into())
      }
      parser::ParseResult::Err(e) => Err(LoadError::ConversionFailed(format!("{:?}", e))),
      _ => Err(LoadError::ConversionFailed("incomplete input".to_owned()))
    }
  }
}

/// Errors that can happen in dependencies.
#[derive(Clone, Debug, PartialEq)]
pub enum DepsError {
  /// If a module’s dependencies has any cycle, the dependencies are unusable and the cycle is
  /// returned.
  Cycle(ModuleKey, ModuleKey),
  /// There was a loading error of a module.
  LoadError(ModuleKey)
}

/// Shader module.
///
/// A shader module is a piece of GLSL code with optional import lists (dependencies).
///
/// You’re not supposed to directly manipulate any object of this type. You just write modules on
/// disk and let everything happen automatically for you.
#[derive(Clone, Debug, PartialEq)]
pub struct Module(syntax::Module);

impl Module {
  /// Retrieve all the modules this module depends on, without duplicates.
  pub fn deps(&self, store: &mut Store, key: &ModuleKey) -> Result<Vec<ModuleKey>, DepsError> {
    let mut deps = Vec::new();
    self.deps_no_cycle(store, &key, &mut Vec::new(), &mut deps).map(|_| deps)
  }

  fn deps_no_cycle(&self, store: &mut Store, key: &ModuleKey, parents: &mut Vec<ModuleKey>, deps: &mut Vec<ModuleKey>) -> Result<(), DepsError> {
    let imports = self.0.imports.iter().map(|il| &il.module);

    parents.push(key.clone());

    for module_path in imports {
      let module_key = ModuleKey(module_path.path.join("."));

      // check whether it’s already in the deps
      if deps.contains(&module_key) {
        continue;
      }

      // check whether the module was already visited
      if parents.contains(&module_key) {
        return Err(DepsError::Cycle(module_key.clone(), module_key.clone()));
      }

      // get the dependency module 
      let module = store.get(&module_key).ok_or_else(|| DepsError::LoadError(module_key.clone()))?;
      module.borrow().deps_no_cycle(store, &module_key, parents, deps)?;

      deps.push(module_key.clone());
      parents.pop();
    }

    Ok(())
  }

  /// Fold a module and its dependencies into a single module. The list of dependencies is also
  /// returned.
  pub fn gather(&self, store: &mut Store, key: &ModuleKey) -> Result<(Self, Vec<ModuleKey>), DepsError> {
    let deps = self.deps(store, key)?;
    let glsl =
      deps.iter()
          .flat_map(|kd| {
              let m = store.get(kd).unwrap();
              let g = m.borrow().0.glsl.clone();
              g
            })
          .chain(self.0.glsl.clone())
          .collect();

    let module = Module(syntax::Module {
      imports: Vec::new(),
      glsl
    });

    Ok((module, deps))
  }

  /// Fold a module into its GLSL setup.
  pub(crate) fn to_glsl_setup(&self) -> Result<ModuleFold, syntax::GLSLConversionError> {
    let uniforms = self.uniforms();
    let blocks = self.blocks();
    let structs = self.structs();
    let functions = self.functions();

    let mut common = String::new();
    let mut vs = String::new();
    let mut gs = String::new();
    let mut fs = String::new();
    let mut structs_str = String::new();

    // sink uniforms, blocks and first as a common framework
    for uniform in &uniforms {
      writer::glsl::show_single_declaration(&mut common, uniform);
      let _ = common.write_str(";\n");
    }

    for block in &blocks {
      writer::glsl::show_block(&mut common, block);
    }

    for f in filter_out_special_functions(functions.iter()) {
      writer::glsl::show_function_definition(&mut common, f)
    }

    let mut filter_out_struct_def = Vec::new();

    // get the special functions
    let map_vertex = functions.iter().find(|fd| &fd.prototype.name == "map_vertex")
                                     .ok_or(syntax::GLSLConversionError::NoVertexShader)?;
    let concat_map_prim = functions.iter().find(|fd| &fd.prototype.name == "concat_map_prim");
    let map_frag_data = functions.iter().find(|fd| &fd.prototype.name == "map_frag_data")
                                        .ok_or(syntax::GLSLConversionError::NoFragmentShader)?;

    // sink the vertex shader
    let (vertex_ret_ty, vertex_outputs) = sink_vertex_shader(&mut vs, map_vertex, &structs)?;
    // since this type has its first field reserved, we must drop it for next stage
    let vertex_ret_ty_fixed = syntax::drop_first_field(&vertex_ret_ty);

    filter_out_struct_def.push(vertex_ret_ty_fixed.name.clone());

    // if there’s any, sink the geometry shader and get its return type – it’ll be passed to the
    // fragment shader; otherwise, just return the vertex type
    let (fs_prev_ret_ty, fs_prev_outputs) = if let Some(concat_map_prim) = concat_map_prim {
      let (ret_ty, outputs) = sink_geometry_shader(&mut gs,
                                                   &concat_map_prim,
                                                   &structs,
                                                   &vertex_ret_ty_fixed,
                                                   &vertex_outputs)?;

      filter_out_struct_def.push(ret_ty.name.clone());
      (ret_ty, outputs)
    } else {
      (vertex_ret_ty_fixed, vertex_outputs)
    };

    // sink the fragment shader
    let (fragment_ret_ty, _) = sink_fragment_shader(&mut fs,
                                                    &map_frag_data,
                                                    &structs,
                                                    &fs_prev_ret_ty,
                                                    &fs_prev_outputs)?;

    filter_out_struct_def.push(fragment_ret_ty.name.clone());

    // filter out structs that might only exist in specific stages
    for s in &structs {
      if !filter_out_struct_def.contains(&s.name) {
        writer::glsl::show_struct(&mut structs_str, s);
      }
    }

    common = structs_str + &common;

    if vs.is_empty() {
      Err(syntax::GLSLConversionError::NoVertexShader)
    } else if fs.is_empty() {
      Err(syntax::GLSLConversionError::NoFragmentShader)
    } else {
      let setup = ModuleFold {
        vs: common.clone() + &vs,
        gs: if gs.is_empty() { None } else { Some(gs.clone()) },
        fs: common.clone() + &fs
      };

      Ok(setup)
    }
  }

  /// Get all the uniforms defined in a module.
  fn uniforms(&self) -> Vec<syntax::SingleDeclaration> {
    let mut uniforms = Vec::new();

    for glsl in &self.0.glsl {
      if let syntax::ExternalDeclaration::Declaration(syntax::Declaration::InitDeclaratorList(ref i)) = *glsl {
        if let Some(ref q) = i.head.ty.qualifier {
          if q.qualifiers.contains(&syntax::TypeQualifierSpec::Storage(syntax::StorageQualifier::Uniform)) {
            uniforms.push(i.head.clone());

            // check whether we have more
            for next in &i.tail {
              uniforms.push(syntax::SingleDeclaration {
                ty: i.head.ty.clone(),
                name: Some(next.name.clone()),
                array_specifier: next.array_specifier.clone(),
                initializer: None
              })
            }
          }
        }
      }
    }

    uniforms
  }

  /// Get all the blocks defined in a module.
  fn blocks(&self) -> Vec<syntax::Block> {
    self.0.glsl.iter().filter_map(|ed| {
      match *ed {
        syntax::ExternalDeclaration::Declaration(syntax::Declaration::Block(ref b)) =>
          Some(b.clone()),
        _ => None
      }
    }).collect()
  }

  /// Get all the functions.
  fn functions(&self) -> Vec<syntax::FunctionDefinition> {
    self.0.glsl.iter().filter_map(|ed| match *ed {
      syntax::ExternalDeclaration::FunctionDefinition(ref def) => Some(def.clone()),
      _ => None
    }).collect()
  }

  /// Get all the declared structures.
  fn structs(&self) -> Vec<syntax::StructSpecifier> {
    self.0.glsl.iter().filter_map(|ed| {
      match *ed {
        syntax::ExternalDeclaration::Declaration(
          syntax::Declaration::InitDeclaratorList(
            syntax::InitDeclaratorList {
              head: syntax::SingleDeclaration {
                ty: syntax::FullySpecifiedType {
                  ty: syntax::TypeSpecifier {
                    ty: syntax::TypeSpecifierNonArray::Struct(ref s),
                    ..
                  },
                  ..
                },
                ..
              },
              ..
            }
          )
        ) => Some(s.clone()),
        _ => None
      }
    }).collect()
  }
}

/// Module fold (pipeline).
///
/// When a module contains all the required functions and structures to define a workable pipeline,
/// it can be folded down to this type, that will be used by lower layers (GPU).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModuleFold {
  pub vs: String,
  pub gs: Option<String>,
  pub fs: String
}

/// Sink a vertex shader.
fn sink_vertex_shader<F>(sink: &mut F,
                         map_vertex: &syntax::FunctionDefinition,
                         structs: &[syntax::StructSpecifier])
                         -> Result<(syntax::StructSpecifier, Vec<syntax::SingleDeclaration>), syntax::GLSLConversionError>
                         where F: Write {
  let inputs = vertex_shader_inputs(&map_vertex.prototype.parameters)?;
  let outputs = vertex_shader_outputs(&map_vertex.prototype.ty, structs)?;
  let ret_ty = syntax::get_fn_ret_ty(map_vertex, structs)?;

  syntax::sink_single_as_ext_decls(sink, inputs.iter().chain(&outputs));

  // sink the return type
  writer::glsl::show_struct(sink, &ret_ty);

  // sink the map_vertex function, but remove its unused arguments
  let map_vertex_reduced = syntax::remove_unused_args_fn(map_vertex);
  writer::glsl::show_function_definition(sink, &map_vertex_reduced);

  // void main
  let _ = sink.write_str("void main() {\n  ");

  // call the map_vertex function
  let mut assigns = String::new();
  sink_vertex_shader_output(sink, &mut assigns, &ret_ty);
  let _ = sink.write_str(" v = map_vertex(");
  sink_vertex_shader_input_args(sink, &map_vertex_reduced);
  let _ = sink.write_str(");\n");

  // assign to outputs
  let _ = sink.write_str(&assigns);

  // end of the main function
  let _ = sink.write_str("}\n\n");

  Ok((ret_ty, outputs))
}

/// Sink a vertex shader’s output.
fn sink_vertex_shader_output<F, G>(sink: &mut F, assigns: &mut G, ty: &syntax::StructSpecifier) where F: Write, G: Write {
  if let Some(ref name) = ty.name {
    let _ = sink.write_str(name);
  } else {
    panic!("cannot happen");
  }

  let _ = assigns.write_str("  gl_Position = v.spsl_Position;\n");

  for field in &ty.fields[1..] {
    for &(ref identifier, _) in &field.identifiers {
      let _ = write!(assigns, "  spsl_v_{0} = v.{0};\n", identifier);
    }
  }
}

/// Sink the arguments of the map_vertex function.
fn sink_vertex_shader_input_args<F>(sink: &mut F, map_vertex: &syntax::FunctionDefinition) where F: Write {
  let args = &map_vertex.prototype.parameters;

  if !args.is_empty() {
    // sink the first argument upfront
    let first_arg = &args[0];

    sink_vertex_shader_input_arg(sink, 0, first_arg);

    for (i, arg) in map_vertex.prototype.parameters[1..].iter().enumerate() {
      if syntax::is_fn_arg_named(arg) {
        let _ = sink.write_str(", ");
        sink_vertex_shader_input_arg(sink, i + 1, arg);
      }
    }
  }
}

/// Sink an argument of a function.
fn sink_vertex_shader_input_arg<F>(sink: &mut F, i: usize, arg: &syntax::FunctionParameterDeclaration) where F: Write {
  match *arg {
    syntax::FunctionParameterDeclaration::Named(_, ref d) => {
      let _ = sink.write_str(&d.name);
    }
    syntax::FunctionParameterDeclaration::Unnamed(..) => {
      let _ = write!(sink, "spsl_unused{}", i);
    }
  }
}

/// Create a vertex’s input (`TypeQualifier`) based on the index and an already `TypeQualifier` of
/// a vertex input.
fn vertex_shader_input_qualifier(i: usize, ty_qual: &Option<syntax::TypeQualifier>) -> syntax::TypeQualifier {
  let layout_qualifier = syntax::LayoutQualifier {
    ids: vec![syntax::LayoutQualifierSpec::Identifier("location".to_owned(),
    Some(Box::new(syntax::Expr::IntConst(i as i32))))]
  };
  let base_qualifier = syntax::TypeQualifier {
    qualifiers: vec![
      syntax::TypeQualifierSpec::Layout(layout_qualifier),
      syntax::TypeQualifierSpec::Storage(syntax::StorageQualifier::In)
    ]
  };

  match *ty_qual {
    Some(ref qual) => syntax::TypeQualifier {
      qualifiers: base_qualifier.qualifiers.into_iter().chain(qual.clone().qualifiers).collect()
    },
    None => base_qualifier
  }
}

/// Extract the vertex shader inputs from a list of arguments.
fn vertex_shader_inputs<'a, I>(args: I) -> Result<Vec<syntax::SingleDeclaration>, syntax::GLSLConversionError>
    where I: IntoIterator<Item = &'a syntax::FunctionParameterDeclaration> {
  let mut inputs = Vec::new();

  for (i, arg) in args.into_iter().enumerate() {
    match *arg {
      syntax::FunctionParameterDeclaration::Named(ref ty_qual, ref decl) => {
        let qualifier = vertex_shader_input_qualifier(i, ty_qual);
        let ty = decl.ty.clone();
        let name = Some(decl.name.clone());
        let array_spec = decl.array_spec.clone();
        let sd = 
          syntax::SingleDeclaration {
            ty: syntax::FullySpecifiedType {
              qualifier: Some(qualifier),
              ty
            },
            name,
            array_specifier: array_spec,
            initializer: None
          };

        inputs.push(sd);
      }

      // unnamed arguments is not an error! it serves when the argument is not used, but we still
      // need to state how the data is stored in the buffer
      _ => ()
    }
  }

  Ok(inputs)
}

fn vertex_shader_outputs(fsty: &syntax::FullySpecifiedType, structs: &[syntax::StructSpecifier]) -> Result<Vec<syntax::SingleDeclaration>, syntax::GLSLConversionError> {
  // we refuse that the output has a main qualifier
  if fsty.qualifier.is_some() {
    return Err(syntax::GLSLConversionError::OutputHasMainQualifier);
  }

  let ty = &fsty.ty;

  // we enforce that the output must be a struct that follows a certain pattern
  match ty.ty {
    syntax::TypeSpecifierNonArray::TypeName(ref ty_name) => {
      let real_ty = structs.iter().find(|ref s| s.name.as_ref() == Some(ty_name));

      match real_ty {
        Some(ref s) => {
          // the first field must be named "spsl_Position", has type vec4 and no qualifier
          let first_field = &s.fields[0];

          if first_field.qualifier.is_some() ||
             first_field.ty.ty != syntax::TypeSpecifierNonArray::Vec4 ||
             first_field.identifiers != vec![("spsl_Position".to_owned(), None)] {
            return Err(syntax::GLSLConversionError::WrongOutputFirstField(first_field.clone()));
          }

          // then, for all other fields, we check that they are not composite type (i.e. structs); if
          // they are not, add them to the interface; otherwise, fail
          syntax::fields_to_single_decls(&s.fields[1..], "spsl_v_")
        }
        _ => Err(syntax::GLSLConversionError::ReturnTypeMustBeAStruct(ty.clone()))
      }
    }
    _ => Err(syntax::GLSLConversionError::ReturnTypeMustBeAStruct(ty.clone()))
  }
}

/// Sink a geometry shader.
fn sink_geometry_shader<F>(
  sink: &mut F,
  concat_map_prim: &syntax::FunctionDefinition,
  structs: &[syntax::StructSpecifier],
  prev_ret_ty: &syntax::StructSpecifier,
  prev_inputs: &[syntax::SingleDeclaration],
) -> Result<(syntax::StructSpecifier, Vec<syntax::SingleDeclaration>),
             syntax::GLSLConversionError>
where F: Write {
  let fn_args = concat_map_prim.prototype.parameters.as_slice();
  let (input_ty_name, input_prim, output_layout_metadata) = match fn_args {
    &[ref arg0, ref arg1] => {
      let input = syntax::fn_arg_as_fully_spec_ty(arg0);
      let output = syntax::fn_arg_as_fully_spec_ty(arg1);

      let input_ty_name = syntax::get_ty_name_from_fully_spec_ty(&input)?;
      let input_prim = guess_gs_input_prim(&input.ty.array_specifier)?;
      let output_layout_metadata = get_gs_output_layout_metadata(&input.qualifier)?;

      Ok((input_ty_name, input_prim, output_layout_metadata))
    }
    _ => Err(syntax::GLSLConversionError::WrongNumberOfArgs(2, fn_args.len()))
  }?;

  // ensure we use the right input type
  if Some(&input_ty_name) != prev_ret_ty.name.as_ref() {
    return Err(syntax::GLSLConversionError::UnknownInputType(input_ty_name.clone()));
  }

  let inputs = syntax::inputs_from_outputs(prev_inputs);
  let ret_ty = syntax::get_fn_ret_ty(concat_map_prim, structs)?;
  let outputs = syntax::fields_to_single_decls(&ret_ty.fields, "spsl_g_")?;

  // syntax::sink_single_as_ext_decls(sink, inputs.iter().chain(&outputs));

  // writer::glsl::show_struct(sink, prev_ret_ty); // sink the previous stage’s return type
  // writer::glsl::show_struct(sink, &ret_ty); // sink the return type of this stage

  // // sink the concat_map_prim function
  // let concat_map_prim_fixed = syntax::remove_unused_args_fn(concat_map_prim);
  // writer::glsl::show_function_definition(sink, &concat_map_prim_fixed);

  // // void main
  // let _ = sink.write_str("void main() {\n  ");

  // let _ = write!(sink, "{0} i = {0}(", prev_ret_ty.name.as_ref().unwrap());

  // let _ = sink.write_str(inputs[0].name.as_ref().unwrap());

  // for input in &inputs[1..] {
  //   let _ = write!(sink, ", {}", input.name.as_ref().unwrap());
  // }

  // let _ = sink.write_str(");\n");
  // let _ = write!(sink, "  {} o = {}(i);\n", ret_ty.name.as_ref().unwrap(), "concat_map_prim");

  // for (output, ret_ty_field) in outputs.iter().zip(&ret_ty.fields) {
  //   let _ = write!(sink, "  {} = o.{};\n", output.name.as_ref().unwrap(), ret_ty_field.identifiers[0].0);
  // }

  // // end of the main function
  // let _ = sink.write_str("}\n\n");

  // Ok((ret_ty, outputs))
}

/// Sink a fragment shader.
fn sink_fragment_shader<F>(sink: &mut F,
                           map_frag_data: &syntax::FunctionDefinition,
                           structs: &[syntax::StructSpecifier],
                           prev_ret_ty: &syntax::StructSpecifier,
                           prev_inputs: &[syntax::SingleDeclaration],
                           ) -> Result<(syntax::StructSpecifier, Vec<syntax::SingleDeclaration>), syntax::GLSLConversionError>
                           where F: Write {
  let input_ty_name = syntax::get_fn1_input_ty_name(map_frag_data)?;

  // ensure we use the right input type
  if Some(&input_ty_name) != prev_ret_ty.name.as_ref() {
    return Err(syntax::GLSLConversionError::UnknownInputType(input_ty_name.clone()));
  }

  let inputs = syntax::inputs_from_outputs(prev_inputs);
  let ret_ty = syntax::get_fn_ret_ty(map_frag_data, structs)?;
  let outputs = syntax::fields_to_single_decls(&ret_ty.fields, "spsl_f_")?;

  syntax::sink_single_as_ext_decls(sink, inputs.iter().chain(&outputs));

  writer::glsl::show_struct(sink, prev_ret_ty); // sink the previous stage’s return type
  writer::glsl::show_struct(sink, &ret_ty); // sink the return type of this stage

  // sink the map_frag_data function
  let map_frag_data_reduced = syntax::remove_unused_args_fn(map_frag_data);
  writer::glsl::show_function_definition(sink, &map_frag_data_reduced);

  // void main
  let _ = sink.write_str("void main() {\n  ");

  let _ = write!(sink, "{0} i = {0}(", prev_ret_ty.name.as_ref().unwrap());

  let _ = sink.write_str(inputs[0].name.as_ref().unwrap());

  for input in &inputs[1..] {
    let _ = write!(sink, ", {}", input.name.as_ref().unwrap());
  }

  let _ = sink.write_str(");\n");
  let _ = write!(sink, "  {} o = {}(i);\n", ret_ty.name.as_ref().unwrap(), "map_frag_data");

  for (output, ret_ty_field) in outputs.iter().zip(&ret_ty.fields) {
    let _ = write!(sink, "  {} = o.{};\n", output.name.as_ref().unwrap(), ret_ty_field.identifiers[0].0);
  }

  // end of the main function
  let _ = sink.write_str("}\n\n");

  Ok((ret_ty, outputs))
}

fn filter_out_special_functions<'a, I>(
  functions: I
) -> impl Iterator<Item = &'a syntax::FunctionDefinition>
where I: Iterator<Item = &'a syntax::FunctionDefinition>
{
  functions.filter(|f| {
    let n: &str = &f.prototype.name;
    n != "map_vertex" && n != "concat_map_prim" && n != "map_frag_data"
  })
}

fn guess_gs_input_prim(array_specifier: &Option<syntax::ArraySpecifier>) -> Result<syntax::LayoutQualifierSpec, syntax::GLSLConversionError> {
  match *array_specifier {
    Some(syntax::ArraySpecifier::ExplicitlySized(box syntax::Expr::IntConst(size))) => {
      match size {
        1 => Ok(syntax::LayoutQualifierSpec::Identifier("points".to_owned(), None)),
        2 => Ok(syntax::LayoutQualifierSpec::Identifier("lines".to_owned(), None)),
        3 => Ok(syntax::LayoutQualifierSpec::Identifier("triangles".to_owned(), None)),
        4 => Ok(syntax::LayoutQualifierSpec::Identifier("lines_adjacency".to_owned(), None)),
        6 => Ok(syntax::LayoutQualifierSpec::Identifier("triangles_adjacency".to_owned(), None)),
        _ => Err(syntax::GLSLConversionError::WrongGeometryInputDim(size as usize))
      }
    },
    _ => Err(syntax::GLSLConversionError::WrongGeometryInput)
  }
}

fn get_gs_output_layout_metadata(qual: &Option<syntax::TypeQualifier>) -> Result<syntax::LayoutQualifier, syntax::GLSLConversionError> {
  let qual = qual.ok_or(syntax::GLSLConversionError::WrongGeometryOutputLayout)?;

  match qual.qualifiers.as_slice() {
    &[syntax::TypeQualifierSpec::Layout(ref layout_qual), syntax::TypeQualifierSpec::Storage(syntax::StorageQualifier::Out)] => {
      match layout_qual.ids.as_slice() {
        &[syntax::LayoutQualifierSpec::Identifier(ref output_prim_str, None),
          syntax::LayoutQualifierSpec::Identifier(ref max_vertices_str, Some(box syntax::Expr::IntConst(max_vertices)))] if max_vertices_str == "max_vertices" => {
          let output_prim = check_gs_output_prim(output_prim_str)?;

          Ok(layout_qual.clone())
        },
        _ => Err(syntax::GLSLConversionError::WrongGeometryOutputLayout)
      }
    },
    _ => Err(syntax::GLSLConversionError::WrongGeometryOutputLayout)
  }
}

fn check_gs_output_prim(s: &str) -> Result<(), syntax::GLSLConversionError> {
  match s {
    "points" | "line_strip" | "triangle_strip" => Ok(()),
    _ => Err(syntax::GLSLConversionError::WrongGeometryOutputLayout)
  }
}
