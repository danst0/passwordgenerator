//! Clipboard writes on GTK's own Wayland connection, using the keyboard-enter
//! serial.
//!
//! GTK passes the serial of the last key/button press it saw to
//! `wl_data_device.set_selection`, never the keyboard-enter serial. Right
//! after startup the app has not received any input yet, so that serial is
//! stale and mutter ignores the selection (it only accepts a selection whose
//! serial is newer than the current one). This module does what `wl-copy`
//! does: wait for `wl_keyboard.enter` and set the selection with its serial.
//! It runs on a private event queue of GTK's connection, so no extra surface
//! is mapped and nothing flickers.

use gtk4::gdk;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::{Rc, Weak};
use std::time::Duration;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::WlDataDeviceManager;
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_data_source::{self, WlDataSource};
use wayland_client::protocol::wl_keyboard::{self, WlKeyboard};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle};

const MIME_TYPES: &[&str] = &[
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "text/plain",
    "TEXT",
    "STRING",
];

/// GTK reads the socket and sorts our events into the private queue; they
/// are handled when we dispatch it. Paste requests (`wl_data_source.send`)
/// arrive the same way, so the queue is polled for the app's lifetime.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Default)]
struct State {
    /// Serial of the last `wl_keyboard.enter`; `None` while unfocused.
    enter_serial: Option<u32>,
    text: Option<String>,
    source: Option<WlDataSource>,
}

pub struct WaylandSelection {
    conn: Connection,
    queue: RefCell<EventQueue<State>>,
    state: RefCell<State>,
    manager: WlDataDeviceManager,
    device: WlDataDevice,
    _keyboard: WlKeyboard,
}

impl WaylandSelection {
    /// Returns `None` when not running on Wayland or the compositor lacks
    /// the required globals; callers then fall back to the GTK clipboard.
    pub fn new(display: &gdk::Display) -> Option<Rc<Self>> {
        let display = display.downcast_ref::<gdk4_wayland::WaylandDisplay>()?;
        let backend = display.wl_display()?.backend().upgrade()?;
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        let seat: WlSeat = globals.bind(&qh, 1..=5, ()).ok()?;
        let manager: WlDataDeviceManager = globals.bind(&qh, 1..=3, ()).ok()?;
        let keyboard = seat.get_keyboard(&qh, ());
        let device = manager.get_data_device(&seat, &qh, ());
        conn.flush().ok()?;

        let this = Rc::new(WaylandSelection {
            conn,
            queue: RefCell::new(queue),
            state: RefCell::new(State::default()),
            manager,
            device,
            _keyboard: keyboard,
        });

        let weak: Weak<Self> = Rc::downgrade(&this);
        glib::timeout_add_local(POLL_INTERVAL, move || match weak.upgrade() {
            Some(this) => {
                this.dispatch();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });

        Some(this)
    }

    fn dispatch(&self) {
        let _ = self
            .queue
            .borrow_mut()
            .dispatch_pending(&mut self.state.borrow_mut());
        let _ = self.conn.flush();
    }

    /// Sets the selection to `text`. Returns false if the app does not hold
    /// keyboard focus (no valid serial), in which case nothing was changed.
    pub fn copy(&self, text: &str) -> bool {
        self.dispatch();
        let mut state = self.state.borrow_mut();
        let Some(serial) = state.enter_serial else {
            return false;
        };
        let qh = self.queue.borrow().handle();
        let source = self.manager.create_data_source(&qh, ());
        for mime in MIME_TYPES {
            source.offer(mime.to_string());
        }
        self.device.set_selection(Some(&source), serial);
        if let Some(old) = state.source.replace(source) {
            old.destroy();
        }
        state.text = Some(text.to_string());
        self.conn.flush().is_ok()
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlSeat, ()> for State {
    fn event(_: &mut Self, _: &WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlDataDeviceManager, ()> for State {
    fn event(_: &mut Self, _: &WlDataDeviceManager, _: <WlDataDeviceManager as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlKeyboard, ()> for State {
    fn event(state: &mut Self, _: &WlKeyboard, event: wl_keyboard::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            wl_keyboard::Event::Enter { serial, .. } => state.enter_serial = Some(serial),
            wl_keyboard::Event::Leave { .. } => state.enter_serial = None,
            // The keymap fd is closed when the event is dropped.
            _ => {}
        }
    }
}

impl Dispatch<WlDataDevice, ()> for State {
    fn event(_: &mut Self, _: &WlDataDevice, event: wl_data_device::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        // We never read from the clipboard; release the offers right away.
        match event {
            wl_data_device::Event::Selection { id: Some(offer) }
            | wl_data_device::Event::Enter { id: Some(offer), .. } => offer.destroy(),
            _ => {}
        }
    }

    event_created_child!(State, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}

impl Dispatch<WlDataOffer, ()> for State {
    fn event(_: &mut Self, _: &WlDataOffer, _: wl_data_offer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlDataSource, ()> for State {
    fn event(state: &mut Self, source: &WlDataSource, event: wl_data_source::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            wl_data_source::Event::Send { fd, .. } => {
                if let Some(text) = &state.text {
                    let _ = File::from(fd).write_all(text.as_bytes());
                }
            }
            wl_data_source::Event::Cancelled => {
                source.destroy();
                if state.source.as_ref() == Some(source) {
                    state.source = None;
                    state.text = None;
                }
            }
            _ => {}
        }
    }
}
