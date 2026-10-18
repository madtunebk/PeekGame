mod analysis;
mod app;
mod cli;
mod loader;
mod model;
mod package;
mod relink;
mod report;
use game_imports::pe;
#[repr(C)]
struct Rlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

unsafe extern "C" {
    fn prctl(option: i32, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32;
    fn getrlimit(resource: i32, rlim: *mut Rlimit) -> i32;
    fn setrlimit(resource: i32, rlim: *const Rlimit) -> i32;
}

fn main() {
    unsafe {
        prctl(0x59616d61, -1isize as usize, 0, 0, 0);
        #[cfg(target_os = "linux")]
        {
            let mut rlim = Rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            const RLIMIT_STACK: i32 = 3;
            if getrlimit(RLIMIT_STACK, &mut rlim) == 0 {
                let desired = 32 * 1024 * 1024;
                if rlim.rlim_cur < desired {
                    rlim.rlim_cur = desired.min(rlim.rlim_max);
                    setrlimit(RLIMIT_STACK, &rlim);
                }
            }
        }
    }
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
