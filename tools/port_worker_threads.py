from pathlib import Path
Path('src/host/thread_template.rs').write_text(r'''use std::sync::OnceLock;
struct Template{base:usize,data:Vec<u8>,callbacks:Vec<usize>}
static TEMPLATE:OnceLock<Template>=OnceLock::new();
pub fn configure(base:usize,data:Vec<u8>,callbacks:Vec<usize>)->Result<(),String>{TEMPLATE.set(Template{base,data,callbacks}).map_err(|_|"thread TLS template already configured".into())}
fn notify(reason:u32){if let Some(t)=TEMPLATE.get(){for &address in &t.callbacks{let callback:extern "win64" fn(usize,u32,usize)=unsafe{std::mem::transmute(address)};callback(t.base,reason,0);}}}
pub fn attach()->Result<(),String>{if let Some(t)=TEMPLATE.get(){super::teb::initialize(t.base,t.data.clone())?;notify(2);}Ok(())}
pub fn detach(){notify(3);}
''')
Path('src/api/threading/threads.rs').write_text(r'''use crate::api::system::{error,startup};
use std::{collections::HashMap,ffi::c_void,sync::{Arc,Mutex,OnceLock,Condvar,atomic::{AtomicUsize,Ordering}},time::{Duration,Instant}};
pub type Start=extern "win64" fn(usize)->u32;
struct Thread{id:u32,exit:Mutex<Option<u32>>,changed:Condvar}
static THREADS:OnceLock<Mutex<HashMap<usize,Arc<Thread>>>>=OnceLock::new();
static NEXT:AtomicUsize=AtomicUsize::new(0x66000000);
fn threads()-> &'static Mutex<HashMap<usize,Arc<Thread>>>{THREADS.get_or_init(||Mutex::new(HashMap::new()))}
fn insert(thread:Arc<Thread>)->usize{let handle=NEXT.fetch_add(16,Ordering::Relaxed);threads().lock().unwrap().insert(handle,thread);handle}
pub unsafe extern "win64" fn create(security:*const c_void,stack:usize,start:Option<Start>,parameter:usize,flags:u32,id:*mut u32)->usize{
 if !security.is_null()||flags!=0{error::set(50);return 0;}
 let Some(start)=start else{error::set(87);return 0;};
 let (send,receive)=std::sync::mpsc::sync_channel(1);
 let mut builder=std::thread::Builder::new();if stack>0{builder=builder.stack_size(stack);}
 let spawned=builder.spawn(move||{
  let thread=Arc::new(Thread{id:startup::thread_id(),exit:Mutex::new(None),changed:Condvar::new()});
  if let Err(e)=crate::host::thread_template::attach(){eprintln!("Windows thread initialization failed: {e}");let _=send.send(None);return;}
  if send.send(Some(thread.clone())).is_err(){return;}
  let result=start(parameter);crate::host::thread_template::detach();
  *thread.exit.lock().unwrap()=Some(result);thread.changed.notify_all();
 });
 if spawned.is_err(){error::set(8);return 0;}
 let Ok(Some(thread))=receive.recv()else{error::set(8);return 0;};
 if !id.is_null(){unsafe{*id=thread.id;}}insert(thread)
}
pub unsafe extern "win64" fn duplicate(source_process:usize,source:usize,target_process:usize,out:*mut usize,access:u32,inherit:i32,options:u32)->i32{
 if out.is_null(){error::set(87);return 0;}unsafe{*out=0;}
 if source_process!=usize::MAX||target_process!=usize::MAX||inherit!=0||options&!3!=0||options&2==0||access!=0{error::set(50);return 0;}
 let target=if source==usize::MAX-1{Some(insert(Arc::new(Thread{id:startup::thread_id(),exit:Mutex::new(None),changed:Condvar::new()})))}
 else if let Some(thread)=threads().lock().unwrap().get(&source).cloned(){Some(insert(thread))}
 else{super::events::duplicate(source)};
 let Some(target)=target else{error::set(6);return 0;};
 unsafe{*out=target;}
 if options&1!=0{super::events::close(source);}1
}
pub fn close(handle:usize)->bool{threads().lock().unwrap().remove(&handle).is_some()}
pub fn wait(handle:usize,milliseconds:u32)->Option<u32>{
 let thread=threads().lock().unwrap().get(&handle).cloned()?;
 let mut exit=thread.exit.lock().unwrap();let start=Instant::now();
 while exit.is_none(){if milliseconds==u32::MAX{exit=thread.changed.wait(exit).unwrap();}
 else{let duration=Duration::from_millis(milliseconds as u64).saturating_sub(start.elapsed());if duration.is_zero(){return Some(258);}exit=thread.changed.wait_timeout(exit,duration).unwrap().0;}}
 Some(0)
}
pub unsafe extern "win64" fn get_exit_code(handle:usize,out:*mut u32)->i32{
 if out.is_null(){error::set(87);return 0;}
 let thread=threads().lock().unwrap().get(&handle).cloned();let Some(thread)=thread else{error::set(6);return 0;};
 unsafe{*out=thread.exit.lock().unwrap().unwrap_or(259);}1
}
''')
p=Path('src/host/mod.rs');p.write_text(p.read_text()+'pub(crate) mod thread_template;\n')
p=Path('src/lib.rs');p.write_text(p.read_text()+'pub use host::thread_template::configure as configure_windows_threads;\n')
p=Path('src/loader/tls.rs');p.write_text(p.read_text().replace('callbacks: Vec<usize>','pub callbacks: Vec<usize>'))
p=Path('src/loader/mod.rs');s=p.read_text().replace('        game_imports::initialize_windows_thread(mapped.base(), tls.data.clone())?;','        game_imports::configure_windows_threads(mapped.base(),tls.data.clone(),tls.callbacks.clone())?;\n        game_imports::initialize_windows_thread(mapped.base(), tls.data.clone())?;');p.write_text(s)
p=Path('src/api/threading/mod.rs');p.write_text(p.read_text()+'pub(crate) mod threads;\n')
p=Path('src/api/threading/events.rs');s=p.read_text();s=s.replace('pub extern "win64" fn close(handle: usize) -> i32 {','pub fn duplicate(handle:usize)->Option<usize>{let event=events().lock().unwrap().get(&handle).cloned()?;let target=NEXT.fetch_add(16,Ordering::Relaxed);events().lock().unwrap().insert(target,event);Some(target)}\npub extern "win64" fn close(handle: usize) -> i32 {');s=s.replace('    if events().lock().unwrap().remove(&handle).is_some() {','    if events().lock().unwrap().remove(&handle).is_some() || super::threads::close(handle) {');s=s.replace('pub extern "win64" fn wait(handle: usize, milliseconds: u32) -> u32 {','pub extern "win64" fn wait(handle: usize, milliseconds: u32) -> u32 {\n    if let Some(result)=super::threads::wait(handle,milliseconds){return result;}');p.write_text(s)
p=Path('src/registry/threading.rs');s=p.read_text().replace('use super::*;','use super::*;\nuse crate::api::threading::threads;').replace('static EXPORTS: &[Export] = &[','static EXPORTS: &[Export] = &[\n implemented!("CreateThread","threading","partial","host threads with Windows TLS; suspended creation unsupported",threads::create),\n implemented!("DuplicateHandle","threading","partial","local event/thread handles; same-access non-inheritable only",threads::duplicate),\n implemented!("GetExitCodeThread","threading","partial","created thread exit code or STILL_ACTIVE",threads::get_exit_code),');p.write_text(s)
