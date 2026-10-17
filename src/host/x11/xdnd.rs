//! Native Xdnd transport only. OLE ownership and callbacks live in api/window.
use super::{ffi, types, Connection};
#[derive(Clone, Copy)]
pub(crate) enum Event {
    Client {
        kind: u8,
        source: u64,
        data: [i64; 5],
    },
    Selection {
        selection: u64,
        target: u64,
        property: u64,
        time: u64,
    },
}
impl Connection {
    pub(crate) fn xdnd_client(&self, e: types::Client) -> Option<Event> {
        if e.format != 32 {
            return None;
        }
        let kind = if e.message_type == self.atom(c"XdndEnter") {
            0
        } else if e.message_type == self.atom(c"XdndPosition") {
            1
        } else if e.message_type == self.atom(c"XdndLeave") {
            2
        } else if e.message_type == self.atom(c"XdndDrop") {
            3
        } else {
            return None;
        };
        Some(Event::Client {
            kind,
            source: e.data[0] as u32 as u64,
            data: e.data,
        })
    }
    pub(crate) fn xdnd_aware(&self, window: u64, enable: bool) {
        unsafe {
            if enable {
                let version = 5u64;
                ffi::XChangeProperty(
                    self.display,
                    window,
                    self.atom(c"XdndAware"),
                    4,
                    32,
                    0,
                    (&version as *const u64).cast(),
                    1,
                );
            } else {
                ffi::XDeleteProperty(self.display, window, self.atom(c"XdndAware"));
            }
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn xdnd_uri(&self) -> u64 {
        self.atom(c"text/uri-list")
    }
    pub(crate) fn xdnd_copy(&self) -> u64 {
        self.atom(c"XdndActionCopy")
    }
    pub(crate) fn xdnd_selection(&self) -> u64 {
        self.atom(c"XdndSelection")
    }
    pub(crate) fn xdnd_property(&self) -> u64 {
        self.atom(c"PeekGameXdndData")
    }
    pub(crate) fn xdnd_can_copy(&self, source: u64, action: u64) -> bool {
        if action == self.xdnd_copy() {
            return true;
        }
        self.xdnd_read(source, self.atom(c"XdndActionList"), 4096)
            .filter(|(kind, format, _)| *kind == 4 && *format == 32)
            .is_some_and(|(_, _, bytes)| {
                bytes
                    .chunks_exact(8)
                    .any(|b| u64::from_ne_bytes(b.try_into().unwrap()) == self.xdnd_copy())
            })
    }
    pub(crate) fn xdnd_types(&self, source: u64) -> Vec<u64> {
        let Some((kind, format, bytes)) = self.xdnd_read(source, self.atom(c"XdndTypeList"), 4096)
        else {
            return Vec::new();
        };
        if kind != 4 || format != 32 {
            return Vec::new();
        }
        bytes
            .chunks_exact(8)
            .map(|b| u64::from_ne_bytes(b.try_into().unwrap()))
            .collect()
    }
    /// Xlib represents format-32 property elements as native unsigned longs.
    pub(crate) fn xdnd_read(
        &self,
        window: u64,
        property: u64,
        max: usize,
    ) -> Option<(u64, i32, Vec<u8>)> {
        unsafe {
            let (mut kind, mut format, mut count, mut remaining, mut data) =
                (0, 0, 0, 0, std::ptr::null_mut());
            let result = ffi::XGetWindowProperty(
                self.display,
                window,
                property,
                0,
                ((max + 3) / 4) as i64,
                0,
                0,
                &mut kind,
                &mut format,
                &mut count,
                &mut remaining,
                &mut data,
            );
            let width = match format {
                8 => 1,
                16 => 2,
                32 => 8,
                _ => 0,
            };
            let length = (count as usize).checked_mul(width);
            let value = if result == 0
                && remaining == 0
                && width != 0
                && length.is_some_and(|n| n <= max * 2)
                && !data.is_null()
            {
                Some((
                    kind,
                    format,
                    std::slice::from_raw_parts(data, length.unwrap()).to_vec(),
                ))
            } else {
                None
            };
            if !data.is_null() {
                ffi::XFree(data.cast());
            }
            value
        }
    }
    pub(crate) fn xdnd_request(&self, window: u64, time: u64) {
        unsafe {
            ffi::XDeleteProperty(self.display, window, self.xdnd_property());
            ffi::XConvertSelection(
                self.display,
                self.xdnd_selection(),
                self.xdnd_uri(),
                self.xdnd_property(),
                window,
                time,
            );
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn xdnd_status(&self, source: u64, window: u64, accept: bool) {
        self.xdnd_send(
            source,
            c"XdndStatus",
            [
                window as i64,
                2 | accept as i64,
                0,
                0,
                if accept { self.xdnd_copy() as i64 } else { 0 },
            ],
        );
    }
    pub(crate) fn xdnd_finished(&self, source: u64, window: u64, ok: bool) {
        self.xdnd_send(
            source,
            c"XdndFinished",
            [
                window as i64,
                ok as i64,
                if ok { self.xdnd_copy() as i64 } else { 0 },
                0,
                0,
            ],
        );
    }
    fn xdnd_send(&self, source: u64, name: &std::ffi::CStr, data: [i64; 5]) {
        unsafe {
            let mut event = types::Event {
                client: types::Client {
                    base: types::Any {
                        kind: 33,
                        serial: 0,
                        send_event: 1,
                        display: self.display,
                        window: source,
                    },
                    message_type: self.atom(name),
                    format: 32,
                    data,
                },
            };
            ffi::XSendEvent(self.display, source, 0, 0, &mut event);
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn xdnd_point(&self, packed: i64) -> (i32, i32, u32) {
        let (ox, oy) = super::monitors::origin(self);
        let (mut root, mut child, mut rx, mut ry, mut x, mut y, mut mask) = (0, 0, 0, 0, 0, 0, 0);
        unsafe {
            ffi::XQueryPointer(
                self.display,
                self.root,
                &mut root,
                &mut child,
                &mut rx,
                &mut ry,
                &mut x,
                &mut y,
                &mut mask,
            );
        }
        let keys = ((mask & 0x100 != 0) as u32)
            | (((mask & 0x400 != 0) as u32) << 1)
            | (((mask & 1 != 0) as u32) << 2)
            | (((mask & 4 != 0) as u32) << 3)
            | (((mask & 0x200 != 0) as u32) << 4)
            | (((mask & 8 != 0) as u32) << 5);
        (
            ((packed as u32 >> 16) as i16 as i32) - ox,
            (packed as i16 as i32) - oy,
            keys,
        )
    }
}
