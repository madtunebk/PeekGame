//! Static TLS for every loaded image. Slot 0 belongs to the EXE; each DLL with a TLS
//! directory takes the first free slot. Every Windows thread has a vector of block
//! pointers (TEB `ThreadLocalStoragePointer`, gs:0x58) indexed by slot. The number of
//! slots follows the images actually loaded: when a vector is too short it is
//! replaced by a longer one and the old array is left allocated, because code on
//! that thread may hold a pointer read from gs:0x58 a moment earlier.
use std::sync::Mutex;

struct ThreadVector {
    /// The TEB's ThreadLocalStoragePointer field.
    field: *mut u64,
    pointers: Vec<usize>,
    blocks: Vec<Option<Box<[u8]>>>,
}
unsafe impl Send for ThreadVector {}
struct State {
    /// Template (initial data plus zero fill) per slot; `None` is a free slot.
    templates: Vec<Option<Vec<u8>>>,
    threads: Vec<ThreadVector>,
}
static STATE: Mutex<State> = Mutex::new(State {
    templates: Vec::new(),
    threads: Vec::new(),
});

impl ThreadVector {
    fn set(&mut self, slot: usize, template: Option<&Vec<u8>>) {
        if slot >= self.pointers.len() {
            let mut longer = Vec::with_capacity((slot + 1).max(self.pointers.len() * 2));
            longer.extend_from_slice(&self.pointers);
            longer.resize(slot + 1, 0);
            unsafe { self.field.write_volatile(longer.as_ptr() as u64) };
            std::mem::forget(std::mem::replace(&mut self.pointers, longer));
            self.blocks.resize_with(slot + 1, || None);
        }
        let block = template.map(|t| t.clone().into_boxed_slice());
        self.pointers[slot] = block.as_ref().map_or(0, |b| b.as_ptr() as usize);
        self.blocks[slot] = block;
    }
}

/// Sets the EXE's template (slot 0) once; later calls keep the first.
pub fn set_exe_template(data: Vec<u8>) {
    let mut state = STATE.lock().unwrap();
    if state.templates.is_empty() {
        state.templates.push(Some(data));
    }
}

/// Gives a DLL a slot, allocates its block in every live thread and returns the slot.
pub fn add_module(template: Vec<u8>) -> usize {
    let mut state = STATE.lock().unwrap();
    if state.templates.is_empty() {
        state.templates.push(None);
    }
    let slot = match state.templates.iter().skip(1).position(Option::is_none) {
        Some(free) => free + 1,
        None => {
            state.templates.push(None);
            state.templates.len() - 1
        }
    };
    state.templates[slot] = Some(template);
    let State { templates, threads } = &mut *state;
    for thread in threads {
        thread.set(slot, templates[slot].as_ref());
    }
    slot
}

/// Frees a DLL's slot and its block in every thread (FreeLibrary).
pub fn remove_module(slot: usize) {
    let mut state = STATE.lock().unwrap();
    if let Some(template) = state.templates.get_mut(slot) {
        *template = None;
    }
    for thread in &mut state.threads {
        if slot < thread.pointers.len() {
            thread.set(slot, None);
        }
    }
}

/// Builds the calling thread's vector with a fresh block for every slot and stores
/// it in `field` (its TEB's ThreadLocalStoragePointer).
pub fn thread_start(field: *mut u64) {
    let mut state = STATE.lock().unwrap();
    let mut vector = ThreadVector {
        field,
        pointers: Vec::new(),
        blocks: Vec::new(),
    };
    // At least one entry, so gs:0x58 is never a dangling empty array.
    vector.set(state.templates.len().max(1) - 1, None);
    for (slot, template) in state.templates.iter().enumerate() {
        vector.set(slot, template.as_ref());
    }
    state.threads.push(vector);
}

/// Frees the blocks of the thread whose TEB field is `field`.
pub fn thread_end(field: *mut u64) {
    let mut state = STATE.lock().unwrap();
    state.threads.retain(|t| t.field != field);
}
