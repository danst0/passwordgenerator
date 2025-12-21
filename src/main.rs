use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Adjustment, Application, ApplicationWindow, Button, CheckButton, CssProvider, Entry, FlowBox, GestureClick, Image, Label, Orientation, PropagationPhase, Revealer, RevealerTransitionType, SelectionMode, SpinButton};
use gio::{Settings, SettingsSchemaSource};
use rand::Rng;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;
use glib::{clone, prelude::Cast, source::SourceId};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const APP_ID: &str = "io.github.danst0.passwordgenerator";
const DEFAULT_GROUPS: i32 = 3;
const CLOSE_AFTER_SEC: i32 = 10;
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SPECIAL: &[u8] = b"!@#$%^&*";

static COLOR_SCHEME_INIT: Once = Once::new();

fn bool_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug)]
struct AppSettings {
    groups: i32,
    auto_close: bool,
    copy_immediately: bool,
    #[serde(default = "bool_true")]
    allow_lowercase: bool,
    #[serde(default = "bool_true")]
    allow_uppercase: bool,
    #[serde(default = "bool_true")]
    allow_digits: bool,
    #[serde(default = "bool_true")]
    allow_special: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            groups: DEFAULT_GROUPS,
            auto_close: true,
            copy_immediately: false,
            allow_lowercase: true,
            allow_uppercase: true,
            allow_digits: true,
            allow_special: true,
        }
    }
}

#[derive(Clone, Copy)]
struct GenerationOptions {
    lowercase: bool,
    uppercase: bool,
    digits: bool,
    special: bool,
}

impl GenerationOptions {
    fn new(lowercase: bool, uppercase: bool, digits: bool, special: bool) -> Self {
        Self {
            lowercase,
            uppercase,
            digits,
            special,
        }
    }

    fn pool(&self) -> Vec<u8> {
        let mut pool = Vec::new();
        if self.lowercase {
            pool.extend_from_slice(LOWER);
        }
        if self.uppercase {
            pool.extend_from_slice(UPPER);
        }
        if self.digits {
            pool.extend_from_slice(DIGITS);
        }
        if self.special {
            pool.extend_from_slice(SPECIAL);
        }
        pool
    }

    fn is_valid(&self) -> bool {
        self.lowercase || self.uppercase || self.digits || self.special
    }
}

#[derive(Clone)]
struct I18nStrings {
    app_title: &'static str,
    groups_tooltip: &'static str,
    generate_button: &'static str,
    copy_button: &'static str,
    auto_close_label: &'static str,
    copy_immediately_label: &'static str,
    timer_template: &'static str,
    copy_success_label: &'static str,
    charset_section_label: &'static str,
    lowercase_label: &'static str,
    uppercase_label: &'static str,
    digits_label: &'static str,
    special_label: &'static str,
    clipboard_log_template: &'static str,
}

impl I18nStrings {
    fn timer_label(&self, seconds: i32) -> String {
        self.timer_template
            .replace("{seconds}", &seconds.to_string())
    }

    fn clipboard_log(&self, password: &str) -> String {
        self.clipboard_log_template
            .replace("{password}", password)
    }
}

fn localized_strings() -> I18nStrings {
    for lang in glib::language_names() {
        if let Some(strings) = strings_for_code(lang.as_str()) {
            return strings;
        }
    }
    strings_en()
}

fn strings_for_code(language: &str) -> Option<I18nStrings> {
    let short = language
        .split(|c| c == '-' || c == '_')
        .next()
        .unwrap_or(language);
    match short {
        "de" => Some(strings_de()),
        "ja" => Some(strings_ja()),
        "sv" => Some(strings_sv()),
        "es" => Some(strings_es()),
        "it" => Some(strings_it()),
        "fr" => Some(strings_fr()),
        "en" => Some(strings_en()),
        _ => None,
    }
}

fn strings_en() -> I18nStrings {
    I18nStrings {
        app_title: "Password Generator",
        groups_tooltip: "Number of groups (5 chars each)",
        generate_button: "New",
        copy_button: "Copy",
        auto_close_label: "Auto-Close",
        copy_immediately_label: "Copy immediately",
        timer_template: "Closes in {seconds}s",
        copy_success_label: "Copied",
        charset_section_label: "Character sets",
        lowercase_label: "Lowercase",
        uppercase_label: "Uppercase",
        digits_label: "Digits",
        special_label: "Special",
        clipboard_log_template: "Copied to clipboard: {password}",
    }
}

fn strings_de() -> I18nStrings {
    I18nStrings {
        app_title: "Passwortgenerator",
        groups_tooltip: "Anzahl Gruppen (je 5 Zeichen)",
        generate_button: "Neu",
        copy_button: "Kopieren",
        auto_close_label: "Auto-Schließen",
        copy_immediately_label: "Sofort kopieren",
        timer_template: "Schließt in {seconds}s",
        copy_success_label: "Kopiert",
        charset_section_label: "Zeichensätze",
        lowercase_label: "Kleinbuchstaben",
        uppercase_label: "Großbuchstaben",
        digits_label: "Ziffern",
        special_label: "Sonderzeichen",
        clipboard_log_template: "In Zwischenablage kopiert: {password}",
    }
}

fn strings_ja() -> I18nStrings {
    I18nStrings {
        app_title: "パスワードジェネレーター",
        groups_tooltip: "グループ数 (5 文字ごと)",
        generate_button: "新規",
        copy_button: "コピー",
        auto_close_label: "自動終了",
        copy_immediately_label: "すぐにコピー",
        timer_template: "あと {seconds} 秒で閉じます",
        copy_success_label: "コピーしました",
        charset_section_label: "文字セット",
        lowercase_label: "小文字",
        uppercase_label: "大文字",
        digits_label: "数字",
        special_label: "記号",
        clipboard_log_template: "クリップボードにコピー: {password}",
    }
}

fn strings_sv() -> I18nStrings {
    I18nStrings {
        app_title: "Lösenordsgenerator",
        groups_tooltip: "Antal grupper (5 tecken vardera)",
        generate_button: "Nytt",
        copy_button: "Kopiera",
        auto_close_label: "Stäng automatiskt",
        copy_immediately_label: "Kopiera direkt",
        timer_template: "Stänger om {seconds}s",
        copy_success_label: "Kopierat",
        charset_section_label: "Teckenuppsättningar",
        lowercase_label: "Gemener",
        uppercase_label: "Versaler",
        digits_label: "Siffror",
        special_label: "Specialtecken",
        clipboard_log_template: "Kopierat till urklipp: {password}",
    }
}

fn strings_es() -> I18nStrings {
    I18nStrings {
        app_title: "Generador de contraseñas",
        groups_tooltip: "Número de grupos (5 caracteres cada uno)",
        generate_button: "Nuevo",
        copy_button: "Copiar",
        auto_close_label: "Cierre automático",
        copy_immediately_label: "Copiar al instante",
        timer_template: "Se cierra en {seconds}s",
        copy_success_label: "Copiado",
        charset_section_label: "Conjuntos de caracteres",
        lowercase_label: "Minúsculas",
        uppercase_label: "Mayúsculas",
        digits_label: "Dígitos",
        special_label: "Caracteres especiales",
        clipboard_log_template: "Copiado al portapapeles: {password}",
    }
}

fn strings_it() -> I18nStrings {
    I18nStrings {
        app_title: "Generatore di password",
        groups_tooltip: "Numero di gruppi (5 caratteri ciascuno)",
        generate_button: "Nuovo",
        copy_button: "Copia",
        auto_close_label: "Chiusura automatica",
        copy_immediately_label: "Copia immediata",
        timer_template: "Si chiude tra {seconds}s",
        copy_success_label: "Copiato",
        charset_section_label: "Set di caratteri",
        lowercase_label: "Minuscole",
        uppercase_label: "Maiuscole",
        digits_label: "Numeri",
        special_label: "Caratteri speciali",
        clipboard_log_template: "Copiato negli appunti: {password}",
    }
}

fn strings_fr() -> I18nStrings {
    I18nStrings {
        app_title: "Générateur de mots de passe",
        groups_tooltip: "Nombre de groupes (5 caractères chacun)",
        generate_button: "Nouveau",
        copy_button: "Copier",
        auto_close_label: "Fermeture auto",
        copy_immediately_label: "Copier immédiatement",
        timer_template: "Fermeture dans {seconds}s",
        copy_success_label: "Copié",
        charset_section_label: "Jeux de caractères",
        lowercase_label: "Minuscules",
        uppercase_label: "Majuscules",
        digits_label: "Chiffres",
        special_label: "Caractères spéciaux",
        clipboard_log_template: "Copié dans le presse-papiers : {password}",
    }
}

fn ensure_system_color_scheme() {
    COLOR_SCHEME_INIT.call_once(|| {
        if let Some(gtk_settings) = gtk::Settings::default() {
            if has_gsettings_schema("org.gnome.desktop.interface") {
                let interface_settings = Settings::new("org.gnome.desktop.interface");
                apply_system_color_preference(&gtk_settings, &interface_settings);
                interface_settings.connect_changed(
                    Some("color-scheme"),
                    clone!(@weak gtk_settings => move |settings, key| {
                        if key == "color-scheme" {
                            apply_system_color_preference(&gtk_settings, settings);
                        }
                    }),
                );
                unsafe {
                    gtk_settings.set_data("system-color-settings", interface_settings);
                }
            }
        }
    });
}

fn has_gsettings_schema(id: &str) -> bool {
    SettingsSchemaSource::default()
        .and_then(|src| src.lookup(id, true))
        .is_some()
}

fn apply_system_color_preference(gtk_settings: &gtk::Settings, interface_settings: &Settings) {
    let prefer_dark = interface_settings
        .string("color-scheme")
        .as_str()
        .eq_ignore_ascii_case("prefer-dark");
    gtk_settings.set_gtk_application_prefer_dark_theme(prefer_dark);
}

fn install_custom_css(window: &ApplicationWindow) {
    let display = gtk::prelude::WidgetExt::display(window);
    let provider = CssProvider::new();

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let css = r#"
        entry.password-entry {
            border-radius: 10px;
            padding: 6px 12px;
        }

        .copy-feedback-box {
            margin-top: -4px;
        }

        .copy-feedback-label,
        .copy-feedback-icon {
            color: @theme_selected_bg_color;
            font-weight: 600;
        }
        "#;

    let _ = provider.load_from_data(css);
}

fn get_config_path() -> PathBuf {
    let mut path = glib::user_config_dir();
    path.push("passwordgenerator");
    std::fs::create_dir_all(&path).unwrap_or_default();
    path.push("settings.json");
    path
}

fn load_settings() -> AppSettings {
    let path = get_config_path();
    if let Ok(file) = fs::File::open(path) {
        if let Ok(settings) = serde_json::from_reader(file) {
            return settings;
        }
    }
    AppSettings::default()
}

fn save_settings(settings: &AppSettings) {
    let path = get_config_path();
    if let Ok(file) = fs::File::create(path) {
        let _ = serde_json::to_writer(file, settings);
    }
}

fn main() {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &Application) {
    let strings = Rc::new(localized_strings());
    let settings = Rc::new(RefCell::new(load_settings()));

    ensure_system_color_scheme();

    let mut needs_charset_save = false;
    {
        let mut config = settings.borrow_mut();
        if !config.allow_lowercase
            && !config.allow_uppercase
            && !config.allow_digits
            && !config.allow_special
        {
            config.allow_lowercase = true;
            needs_charset_save = true;
        }
    }
    if needs_charset_save {
        save_settings(&settings.borrow());
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title(strings.app_title)
        .default_width(420)
        .default_height(320)
        .build();
    install_custom_css(&window);

    let box_container = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(15)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    window.set_child(Some(&box_container));

    let entry = Entry::builder()
        .editable(false)
        .css_classes(vec!["title-3".to_string(), "password-entry".to_string()])
        .build();
    gtk::prelude::EntryExt::set_alignment(&entry, 0.5);
    box_container.append(&entry);

    let controls_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .halign(gtk::Align::Center)
        .build();
    box_container.append(&controls_box);

    let adjustment = Adjustment::new(settings.borrow().groups as f64, 1.0, 10.0, 1.0, 1.0, 0.0);
    let spin_len = SpinButton::new(Some(&adjustment), 1.0, 0);
    spin_len.set_tooltip_text(Some(strings.groups_tooltip));
    controls_box.append(&spin_len);

    let btn_gen = Button::with_label(strings.generate_button);
    controls_box.append(&btn_gen);

    let btn_copy = Button::with_label(strings.copy_button);
    controls_box.append(&btn_copy);

    let copy_feedback_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::Crossfade)
        .reveal_child(false)
        .build();
    let copy_feedback_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::Center)
        .build();
    copy_feedback_box.add_css_class("copy-feedback-box");
    let copy_feedback_icon = Image::from_icon_name("emblem-ok-symbolic");
    copy_feedback_icon.add_css_class("copy-feedback-icon");
    let copy_feedback_label = Label::new(Some(strings.copy_success_label));
    copy_feedback_label.add_css_class("copy-feedback-label");
    copy_feedback_box.append(&copy_feedback_icon);
    copy_feedback_box.append(&copy_feedback_label);
    copy_feedback_revealer.set_child(Some(&copy_feedback_box));
    box_container.append(&copy_feedback_revealer);

    let charset_section = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();
    let charset_label = Label::new(Some(strings.charset_section_label));
    charset_label.set_halign(gtk::Align::Start);
    charset_section.append(&charset_label);

    let charset_flow = FlowBox::builder()
        .column_spacing(12)
        .row_spacing(6)
        .selection_mode(SelectionMode::None)
        .max_children_per_line(2)
        .build();

    let chk_lowercase = CheckButton::with_label(strings.lowercase_label);
    chk_lowercase.set_active(settings.borrow().allow_lowercase);
    charset_flow.insert(&chk_lowercase, -1);

    let chk_uppercase = CheckButton::with_label(strings.uppercase_label);
    chk_uppercase.set_active(settings.borrow().allow_uppercase);
    charset_flow.insert(&chk_uppercase, -1);

    let chk_digits = CheckButton::with_label(strings.digits_label);
    chk_digits.set_active(settings.borrow().allow_digits);
    charset_flow.insert(&chk_digits, -1);

    let chk_special = CheckButton::with_label(strings.special_label);
    chk_special.set_active(settings.borrow().allow_special);
    charset_flow.insert(&chk_special, -1);

    charset_section.append(&charset_flow);
    box_container.append(&charset_section);

    let status_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(15)
        .halign(gtk::Align::Center)
        .build();
    box_container.append(&status_box);

    let chk_auto_close = CheckButton::with_label(strings.auto_close_label);
    chk_auto_close.set_active(settings.borrow().auto_close);
    status_box.append(&chk_auto_close);

    let chk_copy_immediately = CheckButton::with_label(strings.copy_immediately_label);
    chk_copy_immediately.set_active(settings.borrow().copy_immediately);
    status_box.append(&chk_copy_immediately);

    let lbl_timer = Label::new(None);
    status_box.append(&lbl_timer);

    let runtime_auto_close_active = Rc::new(Cell::new(settings.borrow().auto_close));
    let remaining = Rc::new(RefCell::new(CLOSE_AFTER_SEC));

    let gesture = GestureClick::new();
    gesture.set_propagation_phase(PropagationPhase::Capture);
    gesture.connect_pressed(clone!(@weak chk_auto_close, @weak lbl_timer, @strong runtime_auto_close_active => move |_, _, _, _| {
        if chk_auto_close.is_active() && runtime_auto_close_active.get() {
            runtime_auto_close_active.set(false);
            lbl_timer.set_label("");
        }
    }));
    window.add_controller(gesture);

    let pending_copy = Rc::new(RefCell::new(None::<String>));

    let feedback_timeout = Rc::new(RefCell::new(None::<SourceId>));
    let show_copy_feedback: Rc<dyn Fn()> = {
        let revealer = copy_feedback_revealer.clone();
        let timeout_handle = feedback_timeout.clone();
        Rc::new(move || {
            {
                let mut handle = timeout_handle.borrow_mut();
                if let Some(id) = handle.take() {
                    id.remove();
                }
            }
            revealer.set_reveal_child(true);
            let timeout_handle_clone = timeout_handle.clone();
            let revealer_clone = revealer.clone();
            let source_id = glib::timeout_add_local(Duration::from_millis(1500), move || {
                revealer_clone.set_reveal_child(false);
                timeout_handle_clone.borrow_mut().take();
                glib::ControlFlow::Break
            });
            timeout_handle.borrow_mut().replace(source_id);
        })
    };

    let charset_guard = Rc::new(Cell::new(false));
    let charset_buttons = Rc::new(vec![
        chk_lowercase.clone(),
        chk_uppercase.clone(),
        chk_digits.clone(),
        chk_special.clone(),
    ]);

    {
        let settings = settings.clone();
        let guard = charset_guard.clone();
        let buttons = charset_buttons.clone();
        chk_lowercase.connect_toggled(move |btn| {
            if guard.get() {
                return;
            }
            if !buttons.iter().any(|b| b.is_active()) {
                guard.set(true);
                btn.set_active(true);
                guard.set(false);
                return;
            }
            let active = btn.is_active();
            {
                let mut config = settings.borrow_mut();
                config.allow_lowercase = active;
                save_settings(&config);
            }
        });
    }

    {
        let settings = settings.clone();
        let guard = charset_guard.clone();
        let buttons = charset_buttons.clone();
        chk_uppercase.connect_toggled(move |btn| {
            if guard.get() {
                return;
            }
            if !buttons.iter().any(|b| b.is_active()) {
                guard.set(true);
                btn.set_active(true);
                guard.set(false);
                return;
            }
            let active = btn.is_active();
            {
                let mut config = settings.borrow_mut();
                config.allow_uppercase = active;
                save_settings(&config);
            }
        });
    }

    {
        let settings = settings.clone();
        let guard = charset_guard.clone();
        let buttons = charset_buttons.clone();
        chk_digits.connect_toggled(move |btn| {
            if guard.get() {
                return;
            }
            if !buttons.iter().any(|b| b.is_active()) {
                guard.set(true);
                btn.set_active(true);
                guard.set(false);
                return;
            }
            let active = btn.is_active();
            {
                let mut config = settings.borrow_mut();
                config.allow_digits = active;
                save_settings(&config);
            }
        });
    }

    {
        let settings = settings.clone();
        let guard = charset_guard.clone();
        let buttons = charset_buttons;
        chk_special.connect_toggled(move |btn| {
            if guard.get() {
                return;
            }
            if !buttons.iter().any(|b| b.is_active()) {
                guard.set(true);
                btn.set_active(true);
                guard.set(false);
                return;
            }
            let active = btn.is_active();
            {
                let mut config = settings.borrow_mut();
                config.allow_special = active;
                save_settings(&config);
            }
        });
    }

    let update_password = {
        let entry = entry.clone();
        let remaining = remaining.clone();
        let window = window.clone();
        let chk_copy_immediately = chk_copy_immediately.clone();
        let pending_copy = pending_copy.clone();
        let runtime_auto_close_active = runtime_auto_close_active.clone();
        let chk_auto_close = chk_auto_close.clone();
        let chk_lowercase = chk_lowercase.clone();
        let chk_uppercase = chk_uppercase.clone();
        let chk_digits = chk_digits.clone();
        let chk_special = chk_special.clone();
        let show_copy_feedback = show_copy_feedback.clone();
        let strings = strings.clone();
        move |len: i32| {
            let options = GenerationOptions::new(
                chk_lowercase.is_active(),
                chk_uppercase.is_active(),
                chk_digits.is_active(),
                chk_special.is_active(),
            );

            if !options.is_valid() {
                entry.set_text("");
                return;
            }

            let password = generate_password(len, &options);
            entry.set_text(&password);

            if chk_copy_immediately.is_active() {
                if window_is_active(&window) {
                    copy_to_clipboard(&window, &password);
                    println!("{}", strings.clipboard_log(&password));
                    show_copy_feedback();
                    pending_copy.borrow_mut().take();
                } else {
                    *pending_copy.borrow_mut() = Some(password.clone());
                }
            } else {
                pending_copy.borrow_mut().take();
            }

            if runtime_auto_close_active.get() && chk_auto_close.is_active() {
                *remaining.borrow_mut() = CLOSE_AFTER_SEC;
            }
        }
    };

    btn_gen.connect_clicked(clone!(@strong update_password, @weak spin_len => move |_| {
        update_password(spin_len.value() as i32);
    }));

    btn_copy.connect_clicked(clone!(@weak entry, @weak window, @strong strings, @strong show_copy_feedback => move |_| {
        let text = entry.text().to_string();
        copy_to_clipboard(&window, &text);
        println!("{}", strings.clipboard_log(&text));
        show_copy_feedback();
    }));

    spin_len.connect_value_changed(clone!(@strong settings => move |spin| {
        settings.borrow_mut().groups = spin.value() as i32;
        save_settings(&settings.borrow());
    }));

    chk_auto_close.connect_toggled(clone!(@strong settings, @strong remaining, @strong runtime_auto_close_active, @weak lbl_timer => move |chk| {
        let is_active = chk.is_active();
        settings.borrow_mut().auto_close = is_active;
        save_settings(&settings.borrow());

        runtime_auto_close_active.set(is_active);

        if is_active {
            *remaining.borrow_mut() = CLOSE_AFTER_SEC;
        } else {
            lbl_timer.set_label("");
        }
    }));

    chk_copy_immediately.connect_toggled(clone!(@strong settings, @weak entry, @weak window, @strong pending_copy, @strong strings, @strong show_copy_feedback => move |chk| {
        let is_active = chk.is_active();
        settings.borrow_mut().copy_immediately = is_active;
        save_settings(&settings.borrow());

        if is_active {
            let text = entry.text().to_string();
            if window_is_active(&window) {
                copy_to_clipboard(&window, &text);
                println!("{}", strings.clipboard_log(&text));
                show_copy_feedback();
                pending_copy.borrow_mut().take();
            } else {
                *pending_copy.borrow_mut() = Some(text);
            }
        } else {
            pending_copy.borrow_mut().take();
        }
    }));

    let window_weak = window.downgrade();
    let lbl_timer_weak = lbl_timer.downgrade();
    let chk_auto_close_weak = chk_auto_close.downgrade();
    let remaining = remaining.clone();
    let runtime_auto_close_active = runtime_auto_close_active.clone();
    let strings_for_timer = strings.clone();

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

        if !chk_auto_close.is_active() || !runtime_auto_close_active.get() {
            lbl_timer.set_label("");
            return glib::ControlFlow::Continue;
        }

        let mut r = remaining.borrow_mut();
        *r -= 1;
        lbl_timer.set_label(&strings_for_timer.timer_label(*r));

        if *r <= 0 {
            window.close();
            return glib::ControlFlow::Break;
        }

        glib::ControlFlow::Continue
    });

    window.present();

    window.connect_notify_local(
        Some("is-active"),
        clone!(@strong pending_copy, @strong strings, @strong show_copy_feedback => move |win: &ApplicationWindow, _| {
            if window_is_active(win) {
                if let Some(text) = pending_copy.borrow_mut().take() {
                    copy_to_clipboard(win, &text);
                    println!("{}", strings.clipboard_log(&text));
                    show_copy_feedback();
                }
            }
        }),
    );

    glib::idle_add_local_once(clone!(@strong update_password, @strong settings => move || {
        update_password(settings.borrow().groups);
    }));
}

fn generate_password(groups: i32, options: &GenerationOptions) -> String {
    let mut rng = rand::thread_rng();
    let total_groups = groups.max(1);
    let total_chars = total_groups * 5;

    let pool = options.pool();
    if pool.is_empty() {
        return String::new();
    }

    let mut password_chars: Vec<u8> = Vec::with_capacity(total_chars as usize);
    for _ in 0..total_chars {
        let idx = rng.gen_range(0..pool.len());
        password_chars.push(pool[idx]);
    }

    password_chars
        .chunks(5)
        .map(|chunk| chunk.iter().map(|&c| c as char).collect::<String>())
        .collect::<Vec<String>>()
        .join("-")
}

fn copy_to_clipboard(window: &ApplicationWindow, text: &str) {
    let clipboard = gtk::prelude::WidgetExt::display(window).clipboard();
    clipboard.set_text(text);
}

fn window_is_active(window: &ApplicationWindow) -> bool {
    window.upcast_ref::<gtk::Window>().is_active()
}
