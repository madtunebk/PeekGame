//! RandR 1.5 logical monitors; Windows screen coordinates start at the primary.
use super::{ffi, types::Display, Connection};
use std::sync::Mutex;
use std::time::{Duration, Instant};
#[repr(C)]
struct NativeMonitor {
    name: u64,
    primary: i32,
    automatic: i32,
    outputs: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    mm_width: i32,
    mm_height: i32,
    output: *mut u64,
}
#[link(name = "Xrandr")]
unsafe extern "C" {
    fn XRRQueryVersion(display: *mut Display, major: *mut i32, minor: *mut i32) -> i32;
    fn XRRGetMonitors(
        display: *mut Display,
        window: u64,
        active: i32,
        count: *mut i32,
    ) -> *mut NativeMonitor;
    fn XRRFreeMonitors(monitors: *mut NativeMonitor);
}
#[derive(Clone, Debug)]
pub(crate) struct Monitor {
    pub id: u64,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
static CACHE: Mutex<Option<(Instant, Vec<Monitor>)>> = Mutex::new(None);
fn native(c: &Connection) -> Vec<Monitor> {
    let mut cache = CACHE.lock().unwrap();
    if let Some((at, monitors)) = &*cache {
        if at.elapsed() < Duration::from_millis(250) {
            return monitors.clone();
        }
    }
    let mut monitors = Vec::new();
    unsafe {
        let (mut major, mut minor, mut count) = (1, 5, 0);
        if XRRQueryVersion(c.display, &mut major, &mut minor) != 0 && (major, minor) >= (1, 5) {
            let list = XRRGetMonitors(c.display, c.root, 1, &mut count);
            if !list.is_null() {
                for m in std::slice::from_raw_parts(list, count.max(0) as usize) {
                    if m.width > 0 && m.height > 0 {
                        monitors.push(Monitor {
                            id: m.name,
                            primary: m.primary != 0,
                            x: m.x,
                            y: m.y,
                            width: m.width,
                            height: m.height,
                        });
                    }
                }
                XRRFreeMonitors(list);
            }
        }
        if monitors.is_empty() {
            monitors.push(Monitor {
                id: 0,
                primary: true,
                x: 0,
                y: 0,
                width: ffi::XDisplayWidth(c.display, c.screen),
                height: ffi::XDisplayHeight(c.display, c.screen),
            });
        }
    }
    if !monitors.iter().any(|m| m.primary) {
        monitors[0].primary = true;
    }
    monitors.sort_by_key(|m| (!m.primary, m.x, m.y, m.id));
    *cache = Some((Instant::now(), monitors.clone()));
    monitors
}
pub(super) fn origin(c: &Connection) -> (i32, i32) {
    let list = native(c);
    (list[0].x, list[0].y)
}
pub(super) fn list(c: &Connection) -> Vec<Monitor> {
    let mut monitors = native(c);
    let (x, y) = (monitors[0].x, monitors[0].y);
    for m in &mut monitors {
        m.x -= x;
        m.y -= y;
    }
    monitors
}
pub(crate) fn select(monitors: &[Monitor], r: [i32; 4], flags: u32) -> Option<&Monitor> {
    let area = |m: &Monitor| {
        let width = (i64::from(r[2].min(m.x + m.width)) - i64::from(r[0].max(m.x))).max(0);
        let height = (i64::from(r[3].min(m.y + m.height)) - i64::from(r[1].max(m.y))).max(0);
        width * height
    };
    if let Some(m) = monitors
        .iter()
        .max_by_key(|m| area(m))
        .filter(|m| area(m) > 0)
    {
        return Some(m);
    }
    if flags == 1 {
        return monitors.iter().find(|m| m.primary);
    }
    if flags != 2 {
        return None;
    }
    monitors.iter().min_by_key(|m| {
        let dx = (i64::from(m.x) - i64::from(r[2]))
            .max(i64::from(r[0]) - i64::from(m.x + m.width))
            .max(0);
        let dy = (i64::from(m.y) - i64::from(r[3]))
            .max(i64::from(r[1]) - i64::from(m.y + m.height))
            .max(0);
        i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_selection() {
        let monitors = vec![
            Monitor {
                id: 1,
                primary: true,
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
            },
            Monitor {
                id: 2,
                primary: false,
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
            Monitor {
                id: 3,
                primary: false,
                x: 2560,
                y: 0,
                width: 1920,
                height: 1080,
            },
        ];
        assert_eq!(select(&monitors, [-100, 20, -99, 21], 0).unwrap().id, 2);
        assert_eq!(select(&monitors, [-100, 0, 1000, 700], 0).unwrap().id, 1);
        assert!(select(&monitors, [4000, 2000, 4001, 2001], 0).is_none());
        assert_eq!(
            select(&monitors, [4000, 2000, 4001, 2001], 1).unwrap().id,
            1
        );
        assert_eq!(
            select(&monitors, [4000, 2000, 4001, 2001], 2).unwrap().id,
            3
        );
    }
}
