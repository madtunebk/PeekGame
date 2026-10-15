mod analysis;
mod app;
mod cli;
mod loader;
mod model;
mod package;
mod pe;
mod relink;
mod report;
fn main() {
    if let Err(error) = (|| {
        if package::launch_if_packaged()? {
            Ok(())
        } else {
            app::run()
        }
    })() {
        eprintln!("peekgame: {error}");
        std::process::exit(1);
    }
}
