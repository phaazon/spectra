#[macro_use] extern crate spectra;

use spectra::app::demo::{Backbuffer, Builder, Demo, Key, Store, Time};
use spectra::app::runner::debug::Runner;
use spectra::logger::StdoutLogger;

struct App;

impl<Runner> Demo<Runner> for App {
  type Context = StdoutLogger;

  type Error = ();

  fn init(_runner: &mut Runner, _store: &mut Store<Self::Context, Key>, _context: &mut Self::Context) -> Result<Self, Self::Error> {
    Ok(App)
  }

  fn resize(&mut self, _runner: &mut Runner, _context: &mut Self::Context, _width: u32, _height: u32) {
    // do nothing
  }

  fn render(&mut self, _runner: &mut Runner, context: &mut Self::Context, t: Time, _back_buffer: &Backbuffer, _builder: Builder) {
    debug!(context, "time is {}", t);
  }
}

fn main() {
  let mut context = StdoutLogger::default();
  let result = Runner::run::<App>("simple example", 960, 540, &mut context);

  if let Err(e) = result {
    error!(&mut context, "{}", e);
  }

  info!(&mut context, "Bye!");
}
