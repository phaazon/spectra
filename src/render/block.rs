//! Render combinatory blocks.
//!
//! Render blocks are logical rendering units that have inputs and outputs. Inputs can represent
//! vertex attributes, user-specified values, built-ins or previous blocks’ outputs.

use cheddar::Module;
use glsl::syntax::TranslationUnit;
use std::path::PathBuf;
use warmy::{Load, Loaded, Res};

use crate::render::input::Input;
use crate::render::output::Output;

/// A render block, allowing for combining blocks in order to create more complex rendering
/// computations.
#[derive(Clone, Debug)]
struct Block {
  inputs: Vec<Input>,
  outputs: Vec<Output>,
  code_path: PathBuf
}

impl Block {
  /// Create a new block out of inputs, outputs and a GLSL module.
  pub fn new<I, O>(inputs: I, outputs: O, code_path: PathBuf) -> Self
  where I: Iterator<Item = Input>,
        O: Iterator<Item = Output> {
    Block {
      inputs: inputs.collect(),
      outputs: outputs.collect(),
      code_path
    }
  }
}

//impl
