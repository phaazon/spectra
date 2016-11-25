#![feature(const_fn)]
#![feature(proc_macro)]

extern crate gl;
pub extern crate glfw;
extern crate image;
pub extern crate luminance;
pub extern crate luminance_gl;
extern crate nalgebra;
#[cfg(feature = "hot-resource")]
extern crate notify;
extern crate openal;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[cfg(feature = "hot-resource")]
extern crate time;
extern crate vorbis;
extern crate wavefront_obj;

#[macro_use]
pub mod report;

#[macro_use]
pub mod resource;
#[macro_use]
pub mod scene;

pub mod anim;
pub mod app;
pub mod behavior;
pub mod bootstrap;
pub mod camera;
pub mod color;
pub mod device;
pub mod extra;
pub mod id;
pub mod linear;
pub mod model;
pub mod object;
pub mod projection;
pub mod shader;
pub mod spline;
pub mod texture;
pub mod transform;
