use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, CheckButton, Entry, Label, Orientation, SpinButton, Adjustment};
use rand::{Rng, seq::SliceRandom};
use std::cell::RefCell;
use std::rc::Rc;
use glib::clone;

const APP_ID: &str = "io.github.danst0.passwordgenerator";
const DEFAULT_GROUPS: i32 = 3;
const CLOSE_AFTER_SEC: i32 = 10;

fn main() {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Passwortgenerator")
        .default_width(380)
        .default_height(180)
        .build();

    let box_container = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(15)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    window.set_child(Some(&box_container));

    // UI Elements
    let entry = Entry::builder()
        .editable(false)
        .css_classes(vec!["title-3".to_string()])
        .build();
    
    gtk::prelude::EntryExt::set_alignment(&entry, 0.5);
    
    box_container.append(&entry);

    let controls_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(gtk::Align::Center)
        .build();
    
    box_container.append(&controls_box);

    let adjustment = Adjustment::new(DEFAULT_GROUPS as f64, 1.0, 10.0, 1.0, 1.0, 0.0);
    let spin_len = SpinButton::new(Some(&adjustment), 1.0, 0);
    spin_len.set_tooltip_text(Some("Anzahl Gruppen (je 5 Zeichen)"));
    controls_box.append(&spin_len);

    let btn_gen = Button::with_label("Neu");
    controls_box.append(&btn_gen);

    let btn_copy = Button::with_label("Kopieren");
    controls_box.append(&btn_copy);

    let status_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(15)
        .halign(gtk::Align::Center)
        .build();
    box_container.append(&status_box);

    let chk_auto_close = CheckButton::with_label("Auto-Close");
    chk_auto_close.set_active(true);
    status_box.append(&chk_auto_close);

    let lbl_timer = Label::new(None);
    status_box.append(&lbl_timer);

    // Logic
    let remaining = Rc::new(RefCell::new(CLOSE_AFTER_SEC));
    
    // Helper to update password
    let update_password = {
        let entry = entry.clone();
        let remaining = remaining.clone();
        let window = window.clone();
        move |len: i32| {
            let pw = generate_password(len);
            entry.set_text(&pw);
            copy_to_clipboard(&window, &pw);
            *remaining.borrow_mut() = CLOSE_AFTER_SEC;
        }
    };

    // Initial password
    update_password(DEFAULT_GROUPS);

    // Connect signals
    btn_gen.connect_clicked(clone!(@strong update_password, @weak spin_len => move |_| {
        update_password(spin_len.value() as i32);
    }));

    btn_copy.connect_clicked(clone!(@weak entry, @weak window => move |_| {
        copy_to_clipboard(&window, &entry.text());
    }));

    // Timer
    let window_weak = window.downgrade();
    let lbl_timer_weak = lbl_timer.downgrade();
    let chk_auto_close_weak = chk_auto_close.downgrade();
    let remaining = remaining.clone();

    glib::timeout_add_seconds_local(1, move || {
        let window = match window_weak.upgrade() {
            Some(w) => w,
            None => return glib::ControlFlow::Break,
        };
        let lbl_timer = match lbl_timer_weak.upgrade() {
            Some(l) => l,
            None => return glib::ControlFlow::Break,
        };
        let chk_auto_close = match chk_auto_close_weak.upgrade() {
            Some(c) => c,
            None => return glib::ControlFlow::Break,
        };

        if !window.is_visible() {
            return glib::ControlFlow::Break;
        }

        if !chk_auto_close.is_active() {
            lbl_timer.set_label("Timer pausiert");
            return glib::ControlFlow::Continue;
        }

        let mut r = remaining.borrow_mut();
        *r -= 1;
        lbl_timer.set_label(&format!("Schließt in {}s", *r));

        if *r <= 0 {
            window.close();
            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });

    window.present();
}

fn generate_password(groups: i32) -> String {
    const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const SPECIAL: &[u8] = b"!@#$%^&*";

    let mut rng = rand::thread_rng();
    let total_chars = groups * 5;

    // Constraints: 1-2 Upper, 1 Special
    let num_upper = rng.gen_range(1..=2);
    let num_special = 1;
    let num_lower = total_chars - num_upper - num_special;

    let mut password_chars: Vec<u8> = Vec::with_capacity(total_chars as usize);

    for _ in 0..num_upper {
        let idx = rng.gen_range(0..UPPER.len());
        password_chars.push(UPPER[idx]);
    }

    for _ in 0..num_special {
        let idx = rng.gen_range(0..SPECIAL.len());
        password_chars.push(SPECIAL[idx]);
    }

    for _ in 0..num_lower {
        let idx = rng.gen_range(0..LOWER.len());
        password_chars.push(LOWER[idx]);
    }

    password_chars.shuffle(&mut rng);

    password_chars
        .chunks(5)
        .map(|chunk| chunk.iter().map(|&c| c as char).collect::<String>())
        .collect::<Vec<String>>()
        .join("-")
}

fn copy_to_clipboard(window: &ApplicationWindow, text: &str) {
    let clipboard = gtk::prelude::WidgetExt::display(window).clipboard();
    clipboard.set_text(text);
    println!("In Zwischenablage kopiert: {}", text);
}
