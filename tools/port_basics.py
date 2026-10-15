from pathlib import Path
p=Path('src/api/system/startup.rs');p.write_text(r'''use std::{ffi::c_void,sync::atomic::{AtomicUsize,Ordering},time::{SystemTime,UNIX_EPOCH}};
static FILTER:AtomicUsize=AtomicUsize::new(0);
pub extern "win64" fn set_filter(filter:usize)->usize{eprintln!("SetUnhandledExceptionFilter({filter:#x}): recorded; exception dispatch pending");FILTER.swap(filter,Ordering::SeqCst)}
unsafe extern "C"{fn getpid()->i32;fn syscall(number:i64,...)->i64;}
pub extern "win64" fn process_id()->u32{unsafe{getpid() as u32}}
pub extern "win64" fn thread_id()->u32{unsafe{syscall(186) as u32}}
pub extern "win64" fn process()->usize{usize::MAX}
pub extern "win64" fn thread()->usize{usize::MAX-1}
pub extern "win64" fn set_error(code:u32){crate::api::system::error::set(code);}
pub unsafe extern "win64" fn startup_info(p:*mut u8){
 if p.is_null(){return;}unsafe{std::ptr::write_bytes(p,0,104);p.cast::<u32>().write(104);p.add(60).cast::<u32>().write(0x100);}
 if let Some(process)=crate::host::process::PROCESS.get(){for i in 0..3{unsafe{p.add(80+i*8).cast::<usize>().write(process.standard_handles[i]);}}}
}
pub unsafe extern "win64" fn file_time(p:*mut u64){if !p.is_null(){let time=SystemTime::now().duration_since(UNIX_EPOCH).unwrap();let ticks=116444736000000000u64+time.as_secs()*10000000+time.subsec_nanos() as u64/100;unsafe{p.write_unaligned(ticks);}}}
pub unsafe extern "win64" fn performance_frequency(p:*mut i64)->i32{if p.is_null(){crate::api::system::error::set(87);return 0;}unsafe{*p=1000000000;}1}
pub unsafe extern "win64" fn performance_counter(p:*mut i64)->i32{
 if p.is_null(){crate::api::system::error::set(87);return 0;}
 #[repr(C)]struct Timespec{sec:i64,nsec:i64}unsafe extern "C"{fn clock_gettime(clock:i32,time:*mut Timespec)->i32;}
 let mut time=Timespec{sec:0,nsec:0};if unsafe{clock_gettime(1,&mut time)}!=0{crate::api::system::error::set(87);return 0;}unsafe{*p=time.sec*1000000000+time.nsec;}1
}
pub extern "win64" fn sleep(ms:u32){std::thread::sleep(std::time::Duration::from_millis(ms as u64));}
pub extern "win64" fn is_debugger_present()->i32{0}
pub extern "win64" fn get_command_line_w()->*const u16{
 static LINE:std::sync::OnceLock<Vec<u16>>=std::sync::OnceLock::new();
 LINE.get_or_init(||crate::host::process::PROCESS.get().expect("configured process").command_line.to_string_lossy().encode_utf16().chain(Some(0)).collect()).as_ptr()
}
// c_void keeps Windows opaque pointer spelling available to this subsystem.
const _:usize=std::mem::size_of::<*const c_void>();
''')
p=Path('src/api/system/mod.rs');p.write_text(p.read_text()+'pub(crate) mod startup;\n')
functions=[('SetUnhandledExceptionFilter','set_filter','handler registration; dispatch pending'),('GetCurrentProcessId','process_id','host process ID'),('GetCurrentThreadId','thread_id','host thread ID'),('GetCurrentProcess','process','Windows current-process pseudo handle'),('GetCurrentThread','thread','Windows current-thread pseudo handle'),('SetLastError','set_error','thread-local Win32 error'),('GetStartupInfoW','startup_info','inherited streams and default startup fields'),('GetStartupInfoA','startup_info','inherited streams and default startup fields'),('GetSystemTimeAsFileTime','file_time','host UTC file time'),('QueryPerformanceFrequency','performance_frequency','nanosecond host monotonic clock'),('QueryPerformanceCounter','performance_counter','host monotonic clock'),('Sleep','sleep','host thread sleep'),('IsDebuggerPresent','is_debugger_present','no Windows debugger attached'),('GetCommandLineW','get_command_line_w','UTF-16 configured command line')]
p=Path('src/registry/system.rs');s=p.read_text().replace('use super::*;','use super::*;\nuse crate::api::system::startup;');at=s.index('static EXPORTS: &[Export] = &[')+len('static EXPORTS: &[Export] = &[');s=s[:at]+''.join(f'\n implemented!("{n}","system","partial","{scope}",startup::{f}),' for n,f,scope in functions)+s[at:];p.write_text(s)
# Simple functions whose signatures and byte/float semantics match the host C ABI.
items=[('malloc','malloc','size:usize','*mut c_void'),('free','free','ptr:*mut c_void','()'),('calloc','calloc','count:usize,size:usize','*mut c_void'),('realloc','realloc','ptr:*mut c_void,size:usize','*mut c_void'),('memcpy','memcpy','dst:*mut c_void,src:*const c_void,size:usize','*mut c_void'),('memmove','memmove','dst:*mut c_void,src:*const c_void,size:usize','*mut c_void'),('memset','memset','dst:*mut c_void,value:i32,size:usize','*mut c_void'),('memcmp','memcmp','a:*const c_void,b:*const c_void,size:usize','i32'),('memchr','memchr','a:*const c_void,value:i32,size:usize','*mut c_void'),('strlen','strlen','text:*const c_char','usize'),('strcmp','strcmp','a:*const c_char,b:*const c_char','i32'),('strncmp','strncmp','a:*const c_char,b:*const c_char,n:usize','i32'),('strcpy','strcpy','dst:*mut c_char,src:*const c_char','*mut c_char'),('strncpy','strncpy','dst:*mut c_char,src:*const c_char,n:usize','*mut c_char'),('strcat','strcat','dst:*mut c_char,src:*const c_char','*mut c_char'),('strncat','strncat','dst:*mut c_char,src:*const c_char,n:usize','*mut c_char'),('strchr','strchr','text:*const c_char,value:i32','*mut c_char'),('strrchr','strrchr','text:*const c_char,value:i32','*mut c_char'),('strstr','strstr','text:*const c_char,needle:*const c_char','*mut c_char'),('strpbrk','strpbrk','text:*const c_char,accept:*const c_char','*mut c_char'),('strcspn','strcspn','text:*const c_char,reject:*const c_char','usize'),('_strdup','strdup','text:*const c_char','*mut c_char'),('atoi','atoi','text:*const c_char','i32'),('atof','atof','text:*const c_char','f64')]
s='use std::ffi::{c_void,c_char};\n';entries=[]
for index,(win,host,args,result) in enumerate(items):
 names=','.join(arg.split(':')[0] for arg in args.split(','));s+=f'unsafe extern "C" {{#[link_name="{host}"]fn host_{index}({args})->{result};}}\npub unsafe extern "win64" fn call_{index}({args})->{result}{{unsafe{{host_{index}({names})}}}}\n';entries.append(f' crt!("{win}","host memory/byte-string ABI wrapper",simple::call_{index}),')
Path('src/api/crt/simple.rs').write_text(s)
p=Path('src/api/crt/mod.rs');p.write_text(p.read_text()+'pub(crate) mod simple;\n')
p=Path('src/registry/crt.rs');s=p.read_text().replace('use crate::api::crt::startup;','use crate::api::crt::{startup,simple};').replace('static EXPORTS: &[Export] = &[','static EXPORTS: &[Export] = &[\n'+'\n'.join(entries));p.write_text(s)
p=Path('src/api/system/cpu.rs');s=p.read_text().replace('    unsafe {\n','    {\n',1);p.write_text(s)
