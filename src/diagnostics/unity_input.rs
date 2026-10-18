//! Opt-in Mono input inspection, without changing game assemblies or input state.
use std::{
    cell::Cell,
    ffi::{c_char, c_void, CStr},
    sync::atomic::{AtomicU32, Ordering},
};
type P = *mut c_void;
struct Mono {
    foreach: unsafe extern "win64" fn(unsafe extern "win64" fn(P, P), P),
    image: unsafe extern "win64" fn(P) -> P,
    name: unsafe extern "win64" fn(P) -> *const c_char,
    class: unsafe extern "win64" fn(P, *const c_char, *const c_char) -> P,
    method: unsafe extern "win64" fn(P, *const c_char, i32) -> P,
    parent: unsafe extern "win64" fn(P) -> P,
    object_class: unsafe extern "win64" fn(P) -> P,
    invoke: unsafe extern "win64" fn(P, P, *mut P, *mut P) -> P,
    unbox: unsafe extern "win64" fn(P) -> P,
    current: unsafe extern "win64" fn() -> P,
}
impl Mono {
    // The transmute targets are the struct's fn-pointer fields; spelling each one out
    // again at the call site adds noise without clarity.
    #[allow(clippy::missing_transmute_annotations)]
    fn load() -> Option<Self> {
        let module = crate::images::by_name("mono-2.0-bdwgc.dll")?;
        let get = |name| crate::images::export(&module, Some(name), None, 0);
        Some(unsafe {
            Self {
                foreach: std::mem::transmute(get("mono_assembly_foreach")?),
                image: std::mem::transmute(get("mono_assembly_get_image")?),
                name: std::mem::transmute(get("mono_image_get_name")?),
                class: std::mem::transmute(get("mono_class_from_name")?),
                method: std::mem::transmute(get("mono_class_get_method_from_name")?),
                parent: std::mem::transmute(get("mono_class_get_parent")?),
                object_class: std::mem::transmute(get("mono_object_get_class")?),
                invoke: std::mem::transmute(get("mono_runtime_invoke")?),
                unbox: std::mem::transmute(get("mono_object_unbox")?),
                current: std::mem::transmute(get("mono_thread_current")?),
            }
        })
    }
    unsafe fn call(&self, mut class: P, object: P, name: &CStr) -> Option<P> {
        while !class.is_null() {
            let method = unsafe { (self.method)(class, name.as_ptr(), 0) };
            if !method.is_null() {
                let mut exception = std::ptr::null_mut();
                let result =
                    unsafe { (self.invoke)(method, object, std::ptr::null_mut(), &mut exception) };
                return exception.is_null().then_some(result);
            }
            class = unsafe { (self.parent)(class) };
        }
        None
    }
    unsafe fn read(&self, object: P, name: &CStr) -> Option<P> {
        if object.is_null() {
            return None;
        }
        let class = unsafe { (self.object_class)(object) };
        let control = unsafe { self.call(class, object, name) }?;
        if control.is_null() {
            return None;
        }
        let boxed = unsafe { self.call((self.object_class)(control), control, c"ReadValue") }?;
        if boxed.is_null() {
            return None;
        }
        Some(unsafe { (self.unbox)(boxed) })
    }
}
struct Search<'a> {
    mono: &'a Mono,
    image: P,
}
unsafe extern "win64" fn assembly(assembly: P, data: P) {
    let search = unsafe { &mut *data.cast::<Search>() };
    let image = unsafe { (search.mono.image)(assembly) };
    let name = unsafe { (search.mono.name)(image) };
    if !name.is_null()
        && unsafe { CStr::from_ptr(name) }
            .to_bytes()
            .starts_with(b"Unity.InputSystem")
    {
        search.image = image;
    }
}
pub(crate) fn sample() {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ENABLED.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_UNITY_INPUT").is_some()) {
        return;
    }
    thread_local! {static BUSY:Cell<bool>=const{Cell::new(false)};}
    static LAST: AtomicU32 = AtomicU32::new(0);
    let now = crate::host::time::milliseconds();
    let last = LAST.load(Ordering::Relaxed);
    if now.wrapping_sub(last) < 1000
        || LAST
            .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        return;
    }
    BUSY.with(|busy| {
        if busy.replace(true) {
            return;
        }
        unsafe { inspect() };
        busy.set(false);
    });
}
unsafe fn inspect() {
    let Some(mono) = Mono::load() else {
        return;
    };
    if unsafe { (mono.current)() }.is_null() {
        return;
    }
    let mut search = Search {
        mono: &mono,
        image: std::ptr::null_mut(),
    };
    unsafe { (mono.foreach)(assembly, (&mut search as *mut Search).cast()) };
    if search.image.is_null() {
        return;
    }
    let class = unsafe {
        (mono.class)(
            search.image,
            c"UnityEngine.InputSystem".as_ptr(),
            c"Mouse".as_ptr(),
        )
    };
    if class.is_null() {
        return;
    }
    let Some(mouse) = (unsafe { mono.call(class, std::ptr::null_mut(), c"get_current") }) else {
        return;
    };
    if mouse.is_null() {
        eprintln!("UNITY_INPUT: Mouse.current is NULL");
        return;
    }
    let position = unsafe { mono.read(mouse, c"get_position") }.map(|p| unsafe {
        (
            p.cast::<f32>().read_unaligned(),
            p.cast::<f32>().add(1).read_unaligned(),
        )
    });
    let button = unsafe { mono.read(mouse, c"get_leftButton") }
        .map(|p| unsafe { p.cast::<f32>().read_unaligned() });
    let enabled = unsafe { mono.call(class, mouse, c"get_enabled") }
        .filter(|p| !p.is_null())
        .map(|p| unsafe { (mono.unbox)(p).cast::<u8>().read_unaligned() });
    let updated = unsafe { mono.call(class, mouse, c"get_lastUpdateTime") }
        .filter(|p| !p.is_null())
        .map(|p| unsafe { (mono.unbox)(p).cast::<f64>().read_unaligned() });
    eprintln!("UNITY_INPUT: Mouse.current={mouse:p} position={position:?} left={button:?} enabled={enabled:?} updated={updated:?}");
}
