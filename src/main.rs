use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Adjustment, Application, ApplicationWindow, Button, CheckButton, CssProvider, Entry, FlowBox, GestureClick, Label, Orientation, PropagationPhase, Revealer, RevealerTransitionType, SelectionMode, SpinButton};
use gio::{Settings, SettingsSchemaSource, SimpleAction};
use rand::{seq::SliceRandom, Rng, rngs::OsRng};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Once};
use glib::{prelude::Cast, source::SourceId};
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
/// Reference rate for the time-to-crack estimate in the strength tooltip:
/// an offline attack on a fast hash (MD5/NTLM class) with a high-end GPU.
/// Named in the tooltip itself so the number is never presented unqualified.
const GUESSES_PER_SECOND: f64 = 1e12;

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
    #[serde(default)]
    default_strategy: bool,
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
            default_strategy: false,
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

type StringsFactory = fn() -> I18nStrings;

#[derive(Clone)]
struct I18nStrings {
    app_title: &'static str,
    groups_tooltip: &'static str,
    generate_button: &'static str,
    copy_button: &'static str,
    auto_close_label: &'static str,
    copy_immediately_label: &'static str,
    default_strategy_label: &'static str,
    timer_template: &'static str,
    copy_success_label: &'static str,
    charset_section_label: &'static str,
    lowercase_label: &'static str,
    uppercase_label: &'static str,
    digits_label: &'static str,
    special_label: &'static str,
    clipboard_log_template: &'static str,
    chars_unit: &'static str,
    entropy_label: &'static str,
    bits_unit: &'static str,
    crack_time_label: &'static str,
    crack_time_note: &'static str,
    /// Very weak, weak, reasonable, strong, very strong.
    strength_labels: [&'static str; 5],
    /// Seconds, minutes, hours, days, years.
    time_units: [&'static str; 5],
    time_instant: &'static str,
    decimal_separator: &'static str,
}

impl I18nStrings {
    fn timer_label(&self, seconds: i32) -> String {
        self.timer_template
            .replace("{seconds}", &seconds.to_string())
    }

    fn clipboard_log(&self, password: &str) -> String {
        let first = password.chars().next().unwrap_or('?');
        let length = password.chars().count();
        self.clipboard_log_template
            .replace("{first}", &first.to_string())
            .replace("{length}", &length.to_string())
    }

    /// Multi-line tooltip describing the strength of the password on screen.
    fn strength_tooltip(&self, total_chars: usize, bits: f64) -> String {
        format!(
            "{} {} · {} {} {} · {}\n{}: {}\n{}",
            total_chars,
            self.chars_unit,
            self.entropy_label,
            self.format_number(bits, 1),
            self.bits_unit,
            self.strength_labels[strength_tier(bits)],
            self.crack_time_label,
            self.format_duration(crack_time_seconds(bits)),
            self.crack_time_note,
        )
    }

    fn format_number(&self, value: f64, decimals: usize) -> String {
        let text = format!("{:.*}", decimals, value);
        if self.decimal_separator == "." {
            text
        } else {
            text.replace('.', self.decimal_separator)
        }
    }

    /// Expected time-to-crack, scaled to a unit that stays readable across the
    /// whole range (fractions of a second up to 10^70 years).
    fn format_duration(&self, seconds: f64) -> String {
        const MINUTE: f64 = 60.0;
        const HOUR: f64 = 60.0 * MINUTE;
        const DAY: f64 = 24.0 * HOUR;
        const YEAR: f64 = 365.25 * DAY;

        if seconds < 1.0 {
            return self.time_instant.to_string();
        }
        if seconds < MINUTE {
            return format!("{} {}", self.format_number(seconds, 0), self.time_units[0]);
        }
        if seconds < HOUR {
            return format!(
                "{} {}",
                self.format_number(seconds / MINUTE, 0),
                self.time_units[1]
            );
        }
        if seconds < DAY {
            return format!(
                "{} {}",
                self.format_number(seconds / HOUR, 0),
                self.time_units[2]
            );
        }
        if seconds < YEAR {
            return format!(
                "{} {}",
                self.format_number(seconds / DAY, 1),
                self.time_units[3]
            );
        }

        let years = seconds / YEAR;
        if years < 100.0 {
            format!("{} {}", self.format_number(years, 1), self.time_units[4])
        } else if years < 10_000.0 {
            format!("{} {}", self.format_number(years, 0), self.time_units[4])
        } else {
            let mut exponent = years.log10().floor();
            let mut mantissa = years / 10f64.powf(exponent);
            // Keep the mantissa below 10 once rounded to one decimal.
            if mantissa >= 9.95 {
                exponent += 1.0;
                mantissa = years / 10f64.powf(exponent);
            }
            format!(
                "{} × 10{} {}",
                self.format_number(mantissa, 1),
                superscript(exponent as u32),
                self.time_units[4]
            )
        }
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
    AVAILABLE_TRANSLATIONS
        .iter()
        .find(|(code, _)| *code == short)
        .map(|(_, factory)| factory())
}

const AVAILABLE_TRANSLATIONS: &[(&str, StringsFactory)] = &[
    ("de", strings_de),
    ("ja", strings_ja),
    ("sv", strings_sv),
    ("es", strings_es),
    ("it", strings_it),
    ("fr", strings_fr),
    ("en", strings_en),
];

fn strings_en() -> I18nStrings {
    I18nStrings {
        app_title: "Password Generator",
        groups_tooltip: "Number of groups (5 chars each)",
        generate_button: "New",
        copy_button: "Copy",
        auto_close_label: "Auto-Close",
        copy_immediately_label: "Copy immediately",
        default_strategy_label: "Default strategy",
        timer_template: "Closes in {seconds}s",
        copy_success_label: "Copied",
        charset_section_label: "Character sets",
        lowercase_label: "Lowercase",
        uppercase_label: "Uppercase",
        digits_label: "Digits",
        special_label: "Special",
        clipboard_log_template: "Copied to clipboard: first '{first}', length {length}",
        chars_unit: "characters",
        entropy_label: "Entropy",
        bits_unit: "bits",
        crack_time_label: "Offline attack",
        crack_time_note: "Assumes 10¹² guesses/s (fast hash, high-end GPU)",
        strength_labels: ["Very weak", "Weak", "Reasonable", "Strong", "Very strong"],
        time_units: ["s", "min", "h", "days", "years"],
        time_instant: "instantly",
        decimal_separator: ".",
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
        default_strategy_label: "Standardstrategie",
        timer_template: "Schließt in {seconds}s",
        copy_success_label: "Kopiert",
        charset_section_label: "Zeichensätze",
        lowercase_label: "Kleinbuchstaben",
        uppercase_label: "Großbuchstaben",
        digits_label: "Ziffern",
        special_label: "Sonderzeichen",
        clipboard_log_template: "In Zwischenablage kopiert: erster Buchstabe '{first}', Länge {length}",
        chars_unit: "Zeichen",
        entropy_label: "Entropie",
        bits_unit: "Bit",
        crack_time_label: "Offline-Angriff",
        crack_time_note: "Annahme: 10¹² Versuche/s (schneller Hash, High-End-GPU)",
        strength_labels: [
            "Sehr schwach",
            "Schwach",
            "Angemessen",
            "Stark",
            "Sehr stark",
        ],
        time_units: ["s", "Min.", "Std.", "Tage", "Jahre"],
        time_instant: "sofort",
        decimal_separator: ",",
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
        default_strategy_label: "デフォルト戦略",
        timer_template: "あと {seconds} 秒で閉じます",
        copy_success_label: "コピーしました",
        charset_section_label: "文字セット",
        lowercase_label: "小文字",
        uppercase_label: "大文字",
        digits_label: "数字",
        special_label: "記号",
        clipboard_log_template: "クリップボードにコピー: 先頭 '{first}', 長さ {length}",
        chars_unit: "文字",
        entropy_label: "エントロピー",
        bits_unit: "ビット",
        crack_time_label: "オフライン攻撃",
        crack_time_note: "前提: 毎秒 10¹² 回の試行 (高速ハッシュ、ハイエンド GPU)",
        strength_labels: ["非常に弱い", "弱い", "標準的", "強い", "非常に強い"],
        time_units: ["秒", "分", "時間", "日", "年"],
        time_instant: "即座に",
        decimal_separator: ".",
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
        default_strategy_label: "Standardstrategi",
        timer_template: "Stänger om {seconds}s",
        copy_success_label: "Kopierat",
        charset_section_label: "Teckenuppsättningar",
        lowercase_label: "Gemener",
        uppercase_label: "Versaler",
        digits_label: "Siffror",
        special_label: "Specialtecken",
        clipboard_log_template: "Kopierat till urklipp: första '{first}', längd {length}",
        chars_unit: "tecken",
        entropy_label: "Entropi",
        bits_unit: "bitar",
        crack_time_label: "Offlineattack",
        crack_time_note: "Antagande: 10¹² gissningar/s (snabb hash, high-end-GPU)",
        strength_labels: [
            "Mycket svagt",
            "Svagt",
            "Godtagbart",
            "Starkt",
            "Mycket starkt",
        ],
        time_units: ["s", "min", "h", "dagar", "år"],
        time_instant: "omedelbart",
        decimal_separator: ",",
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
        default_strategy_label: "Estrategia predeterminada",
        timer_template: "Se cierra en {seconds}s",
        copy_success_label: "Copiado",
        charset_section_label: "Conjuntos de caracteres",
        lowercase_label: "Minúsculas",
        uppercase_label: "Mayúsculas",
        digits_label: "Dígitos",
        special_label: "Caracteres especiales",
        clipboard_log_template: "Copiado al portapapeles: primera '{first}', longitud {length}",
        chars_unit: "caracteres",
        entropy_label: "Entropía",
        bits_unit: "bits",
        crack_time_label: "Ataque offline",
        crack_time_note: "Supone 10¹² intentos/s (hash rápido, GPU de gama alta)",
        strength_labels: ["Muy débil", "Débil", "Aceptable", "Fuerte", "Muy fuerte"],
        time_units: ["s", "min", "h", "días", "años"],
        time_instant: "al instante",
        decimal_separator: ",",
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
        default_strategy_label: "Strategia predefinita",
        timer_template: "Si chiude tra {seconds}s",
        copy_success_label: "Copiato",
        charset_section_label: "Set di caratteri",
        lowercase_label: "Minuscole",
        uppercase_label: "Maiuscole",
        digits_label: "Numeri",
        special_label: "Caratteri speciali",
        clipboard_log_template: "Copiato negli appunti: prima '{first}', lunghezza {length}",
        chars_unit: "caratteri",
        entropy_label: "Entropia",
        bits_unit: "bit",
        crack_time_label: "Attacco offline",
        crack_time_note: "Ipotesi: 10¹² tentativi/s (hash veloce, GPU di fascia alta)",
        strength_labels: [
            "Molto debole",
            "Debole",
            "Accettabile",
            "Forte",
            "Molto forte",
        ],
        time_units: ["s", "min", "h", "giorni", "anni"],
        time_instant: "istantaneo",
        decimal_separator: ",",
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
        default_strategy_label: "Stratégie par défaut",
        timer_template: "Fermeture dans {seconds}s",
        copy_success_label: "Copié",
        charset_section_label: "Jeux de caractères",
        lowercase_label: "Minuscules",
        uppercase_label: "Majuscules",
        digits_label: "Chiffres",
        special_label: "Caractères spéciaux",
        clipboard_log_template: "Copié dans le presse-papiers : première '{first}', longueur {length}",
        chars_unit: "caractères",
        entropy_label: "Entropie",
        bits_unit: "bits",
        crack_time_label: "Attaque hors ligne",
        crack_time_note: "Hypothèse : 10¹² essais/s (hachage rapide, GPU haut de gamme)",
        strength_labels: ["Très faible", "Faible", "Acceptable", "Fort", "Très fort"],
        time_units: ["s", "min", "h", "jours", "ans"],
        time_instant: "instantané",
        decimal_separator: ",",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_cover_all_languages() {
        for (code, factory) in AVAILABLE_TRANSLATIONS {
            let strings = factory();
            assert!(
                !strings.app_title.is_empty()
                    && !strings.generate_button.is_empty()
                    && !strings.copy_button.is_empty()
                    && !strings.default_strategy_label.is_empty(),
                "Missing strings for language code {}",
                code
            );
            assert!(
                !strings.chars_unit.is_empty()
                    && !strings.entropy_label.is_empty()
                    && !strings.bits_unit.is_empty()
                    && !strings.crack_time_label.is_empty()
                    && !strings.crack_time_note.is_empty()
                    && !strings.time_instant.is_empty()
                    && strings.strength_labels.iter().all(|s| !s.is_empty())
                    && strings.time_units.iter().all(|s| !s.is_empty()),
                "Missing strength tooltip strings for language code {}",
                code
            );
            assert!(
                strings.decimal_separator == "." || strings.decimal_separator == ",",
                "Unexpected decimal separator for language code {}",
                code
            );
        }
    }

    /// Does a password match the shape the default strategy always produces:
    /// exactly one character from each enabled extra class, everything else
    /// lowercase?
    fn matches_default_shape(word: &[u8], options: &GenerationOptions) -> bool {
        let occurrences = |set: &[u8]| word.iter().filter(|c| set.contains(c)).count();
        let mut extras = 0;
        for (enabled, set) in [
            (options.uppercase, UPPER),
            (options.digits, DIGITS),
            (options.special, SPECIAL),
        ] {
            let found = occurrences(set);
            if enabled {
                if found != 1 {
                    return false;
                }
                extras += 1;
            } else if found != 0 {
                return false;
            }
        }
        occurrences(LOWER) == word.len() - extras
    }

    /// Count the reachable passwords by walking the whole search space, without
    /// reusing any of the maths from `entropy_bits_for_length`.
    fn count_reachable_by_enumeration(total_chars: usize, options: &GenerationOptions) -> u64 {
        let mut alphabet: Vec<u8> = LOWER.to_vec();
        if options.uppercase {
            alphabet.extend_from_slice(UPPER);
        }
        if options.digits {
            alphabet.extend_from_slice(DIGITS);
        }
        if options.special {
            alphabet.extend_from_slice(SPECIAL);
        }

        let base = alphabet.len();
        let mut indices = vec![0usize; total_chars];
        let mut word = vec![0u8; total_chars];
        let mut count = 0u64;
        loop {
            for (slot, &i) in word.iter_mut().zip(indices.iter()) {
                *slot = alphabet[i];
            }
            if matches_default_shape(&word, options) {
                count += 1;
            }

            let mut pos = 0;
            while pos < total_chars {
                indices[pos] += 1;
                if indices[pos] < base {
                    break;
                }
                indices[pos] = 0;
                pos += 1;
            }
            if pos == total_chars {
                return count;
            }
        }
    }

    #[test]
    fn default_strategy_entropy_matches_enumerated_keyspace() {
        // Two enabled extra classes: 62^3 candidates.
        let two_classes = GenerationOptions::new(true, true, true, false);
        // All three enabled, every position forced: 70^3 candidates.
        let three_classes = GenerationOptions::new(true, true, true, true);

        for options in [two_classes, three_classes] {
            let expected = (count_reachable_by_enumeration(3, &options) as f64).log2();
            let actual = entropy_bits_for_length(3, &options, true).expect("entropy available");
            assert!(
                (actual - expected).abs() < 1e-9,
                "entropy {actual} does not match enumerated keyspace {expected}"
            );
        }
    }

    #[test]
    fn generated_passwords_match_the_shape_the_entropy_assumes() {
        let options = GenerationOptions::new(true, true, true, true);
        for _ in 0..200 {
            let password = generate_password(3, &options, true);
            let word: Vec<u8> = password.bytes().filter(|b| *b != b'-').collect();
            assert_eq!(word.len(), 15);
            assert!(
                matches_default_shape(&word, &options),
                "unexpected password shape: {password}"
            );
        }
    }

    #[test]
    fn custom_strategy_entropy_is_length_times_pool() {
        let options = GenerationOptions::new(true, true, true, true);
        let bits = password_entropy_bits(3, &options, false).expect("entropy available");
        assert!((bits - 15.0 * 70f64.log2()).abs() < 1e-9);

        let lowercase_only = GenerationOptions::new(true, false, false, false);
        let bits = password_entropy_bits(1, &lowercase_only, false).expect("entropy available");
        assert!((bits - 5.0 * 26f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn entropy_is_unavailable_without_any_character_set() {
        let empty = GenerationOptions::new(false, false, false, false);
        assert!(password_entropy_bits(3, &empty, false).is_none());
    }

    #[test]
    fn default_strategy_is_weaker_than_the_naive_estimate() {
        // The mostly-lowercase base makes the default strategy weaker than
        // "length × log2(full pool)" would suggest — the tooltip must not
        // overstate it.
        let options = GenerationOptions::new(true, true, true, true);
        let actual = password_entropy_bits(3, &options, true).expect("entropy available");
        let naive = 15.0 * 70f64.log2();
        assert!(actual < naive - 10.0, "default strategy entropy: {actual}");
        assert!(actual > 15.0 * 26f64.log2());
    }

    #[test]
    fn durations_scale_across_the_whole_range() {
        let en = strings_en();
        assert_eq!(en.format_duration(0.5), "instantly");
        assert_eq!(en.format_duration(30.0), "30 s");
        assert_eq!(en.format_duration(600.0), "10 min");
        assert_eq!(en.format_duration(7200.0), "2 h");
        assert_eq!(en.format_duration(3.0 * 86_400.0), "3.0 days");

        // Large values fall back to scientific notation, which needs no
        // translation, and the mantissa always stays below 10.
        let huge = en.format_duration(crack_time_seconds(200.0));
        assert!(huge.contains("× 10"), "unexpected duration: {huge}");
        assert!(huge.ends_with("years"), "unexpected duration: {huge}");
        for bits in 30..310 {
            let text = en.format_duration(crack_time_seconds(bits as f64));
            assert!(!text.contains("10.0 ×"), "bad mantissa at {bits} bits: {text}");
            assert!(!text.contains("inf") && !text.contains("NaN"), "{text}");
        }
    }

    #[test]
    fn german_tooltip_uses_a_decimal_comma() {
        let de = strings_de();
        let options = GenerationOptions::new(true, true, true, true);
        let bits = password_entropy_bits(3, &options, false).expect("entropy available");
        let tooltip = de.strength_tooltip(15, bits);
        assert!(tooltip.contains("91,9 Bit"), "unexpected tooltip: {tooltip}");
        assert!(tooltip.contains("· Stark"), "unexpected tooltip: {tooltip}");
        assert!(
            tooltip.contains("7,5 × 10⁷ Jahre"),
            "unexpected tooltip: {tooltip}"
        );
        assert_eq!(tooltip.lines().count(), 3);
    }

    #[test]
    fn strength_tiers_cover_the_configurable_range() {
        let all = GenerationOptions::new(true, true, true, true);
        let lowercase_only = GenerationOptions::new(true, false, false, false);
        // Weakest possible setting: one group of lowercase characters.
        assert_eq!(
            strength_tier(password_entropy_bits(1, &lowercase_only, false).unwrap()),
            0
        );
        // Strongest: ten groups over the full pool.
        assert_eq!(
            strength_tier(password_entropy_bits(10, &all, false).unwrap()),
            4
        );
    }

    #[test]
    fn superscript_renders_exponents() {
        assert_eq!(superscript(0), "⁰");
        assert_eq!(superscript(7), "⁷");
        assert_eq!(superscript(72), "⁷²");
        assert_eq!(superscript(104), "¹⁰⁴");
    }
}

fn ensure_system_color_scheme() {
    COLOR_SCHEME_INIT.call_once(|| {
        if let Some(gtk_settings) = gtk::Settings::default() {
            if has_gsettings_schema("org.gnome.desktop.interface") {
                let interface_settings = Settings::new("org.gnome.desktop.interface");
                apply_system_color_preference(&gtk_settings, &interface_settings);
                let gtk_settings_for_change = gtk_settings.clone();
                interface_settings.connect_changed(Some("color-scheme"), move |settings, key| {
                    if key == "color-scheme" {
                        apply_system_color_preference(&gtk_settings_for_change, settings);
                    }
                });
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

    let app_weak = app.downgrade();
    let quit_action = SimpleAction::new("quit", None);
    quit_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit_action);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

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

    let window_weak = window.downgrade();
    let close_action = SimpleAction::new("close", None);
    close_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    window.add_action(&close_action);
    app.set_accels_for_action("win.close", &["<Control>w"]);

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
    let copy_feedback_icon = Label::new(Some("✅"));
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

    let chk_default_strategy = CheckButton::with_label(strings.default_strategy_label);
    chk_default_strategy.set_active(settings.borrow().default_strategy);
    status_box.append(&chk_default_strategy);

    let lbl_timer = Label::new(None);
    status_box.append(&lbl_timer);

    let runtime_auto_close_active = Rc::new(Cell::new(settings.borrow().auto_close));
    let remaining = Rc::new(RefCell::new(CLOSE_AFTER_SEC));

    let gesture = GestureClick::new();
    gesture.set_propagation_phase(PropagationPhase::Capture);
    let chk_auto_close_weak = chk_auto_close.downgrade();
    let lbl_timer_weak = lbl_timer.downgrade();
    let runtime_auto_close_active_for_gesture = runtime_auto_close_active.clone();
    gesture.connect_pressed(move |_, _, _, _| {
        if let (Some(chk_auto_close), Some(lbl_timer)) = (
            chk_auto_close_weak.upgrade(),
            lbl_timer_weak.upgrade(),
        ) {
            if chk_auto_close.is_active() && runtime_auto_close_active_for_gesture.get() {
                runtime_auto_close_active_for_gesture.set(false);
                lbl_timer.set_label("");
            }
        }
    });
    window.add_controller(gesture);

    let copy_state = CopyState::new();

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
        let default_strategy_btn = chk_default_strategy.clone();
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

            if !active && default_strategy_btn.is_active() {
                default_strategy_btn.set_active(false);
            }

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
        let chk_default_strategy = chk_default_strategy.clone();
        let copy_state = copy_state.clone();
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

            let use_default_strategy = chk_default_strategy.is_active();

            if !use_default_strategy && !options.is_valid() {
                entry.set_text("");
                entry.set_tooltip_text(None);
                return;
            }

            let password = generate_password(len, &options, use_default_strategy);
            entry.set_text(&password);

            // Describe the password on screen in a tooltip, so the strength
            // information costs no permanent screen real estate.
            match password_entropy_bits(len, &options, use_default_strategy) {
                Some(bits) if !password.is_empty() => {
                    let total_chars = password.chars().filter(|c| *c != '-').count();
                    entry.set_tooltip_text(Some(&strings.strength_tooltip(total_chars, bits)));
                }
                _ => entry.set_tooltip_text(None),
            }

            if chk_copy_immediately.is_active() {
                schedule_auto_copy(
                    &window,
                    copy_state.clone(),
                    show_copy_feedback.clone(),
                    strings.clone(),
                    password.clone(),
                );
            } else {
                copy_state.cancel();
            }

            if runtime_auto_close_active.get() && chk_auto_close.is_active() {
                *remaining.borrow_mut() = CLOSE_AFTER_SEC;
            }
        }
    };

    let update_password_for_button = update_password.clone();
    let spin_len_weak = spin_len.downgrade();
    btn_gen.connect_clicked(move |_| {
        if let Some(spin_len) = spin_len_weak.upgrade() {
            update_password_for_button(spin_len.value() as i32);
        }
    });

    let entry_weak_for_copy = entry.downgrade();
    let window_weak_for_copy = window.downgrade();
    let strings_for_copy = strings.clone();
    let copy_state_for_button = copy_state.clone();
    let show_copy_feedback_for_button = show_copy_feedback.clone();
    btn_copy.connect_clicked(move |_| {
        if let (Some(entry), Some(window)) = (
            entry_weak_for_copy.upgrade(),
            window_weak_for_copy.upgrade(),
        ) {
            let text = entry.text().to_string();
            if !window_is_active(&window) {
                window.present();
            }
            schedule_auto_copy(
                &window,
                copy_state_for_button.clone(),
                show_copy_feedback_for_button.clone(),
                strings_for_copy.clone(),
                text,
            );
        }
    });

    let settings_for_spin = settings.clone();
    spin_len.connect_value_changed(move |spin| {
        settings_for_spin.borrow_mut().groups = spin.value() as i32;
        save_settings(&settings_for_spin.borrow());
    });

    let settings_for_auto_close = settings.clone();
    let remaining_for_auto_close = remaining.clone();
    let runtime_auto_close_flag = runtime_auto_close_active.clone();
    let lbl_timer_weak = lbl_timer.downgrade();
    chk_auto_close.connect_toggled(move |chk| {
        let is_active = chk.is_active();
        settings_for_auto_close.borrow_mut().auto_close = is_active;
        save_settings(&settings_for_auto_close.borrow());

        runtime_auto_close_flag.set(is_active);

        if is_active {
            *remaining_for_auto_close.borrow_mut() = CLOSE_AFTER_SEC;
        } else if let Some(lbl_timer) = lbl_timer_weak.upgrade() {
            lbl_timer.set_label("");
        }
    });

    let settings_for_copy_toggle = settings.clone();
    let entry_weak_for_toggle = entry.downgrade();
    let window_weak_for_toggle = window.downgrade();
    let copy_state_for_toggle = copy_state.clone();
    let strings_for_copy_toggle = strings.clone();
    let show_copy_feedback_for_toggle = show_copy_feedback.clone();
    chk_copy_immediately.connect_toggled(move |chk| {
        let is_active = chk.is_active();
        settings_for_copy_toggle.borrow_mut().copy_immediately = is_active;
        save_settings(&settings_for_copy_toggle.borrow());

        if is_active {
            if let (Some(entry), Some(window)) = (
                entry_weak_for_toggle.upgrade(),
                window_weak_for_toggle.upgrade(),
            ) {
                let text = entry.text().to_string();
                schedule_auto_copy(
                    &window,
                    copy_state_for_toggle.clone(),
                    show_copy_feedback_for_toggle.clone(),
                    strings_for_copy_toggle.clone(),
                    text,
                );
            }
        } else {
            copy_state_for_toggle.cancel();
        }
    });

    let settings_for_strategy = settings.clone();
    let chk_lowercase_for_strategy = chk_lowercase.clone();
    chk_default_strategy.connect_toggled(move |chk| {
        let is_active = chk.is_active();

        if is_active && !chk_lowercase_for_strategy.is_active() {
            chk_lowercase_for_strategy.set_active(true);
        }

        settings_for_strategy.borrow_mut().default_strategy = is_active;
        save_settings(&settings_for_strategy.borrow());
    });

    let window_weak = window.downgrade();
    let lbl_timer_weak = lbl_timer.downgrade();
    let chk_auto_close_weak = chk_auto_close.downgrade();
    let remaining = remaining.clone();
    let runtime_auto_close_active = runtime_auto_close_active.clone();
    let strings_for_timer = strings.clone();
    let copy_state_for_timer = copy_state.clone();

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

        // Never close before a pending clipboard copy has completed
        // (e.g. window has not received focus yet on Wayland).
        if copy_state_for_timer.pending.borrow().is_some() {
            *remaining.borrow_mut() = CLOSE_AFTER_SEC;
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

    // Track real keyboard focus and copy the moment it arrives: right after
    // the keyboard-enter event the Wayland input serial is fresh, so the
    // clipboard write is guaranteed to be accepted. Registered BEFORE
    // present() so the initial focus is never missed.
    let focus_controller = gtk::EventControllerFocus::new();
    let copy_state_for_focus = copy_state.clone();
    let window_weak_for_focus = window.downgrade();
    let strings_for_focus = strings.clone();
    let show_copy_feedback_for_focus = show_copy_feedback.clone();
    focus_controller.connect_enter(move |_| {
        copy_state_for_focus.keyboard_focus.set(true);
        if let Some(window) = window_weak_for_focus.upgrade() {
            try_pending_copy(
                &window,
                &copy_state_for_focus,
                &show_copy_feedback_for_focus,
                &strings_for_focus,
            );
        }
    });
    let copy_state_for_blur = copy_state.clone();
    focus_controller.connect_leave(move |_| {
        copy_state_for_blur.keyboard_focus.set(false);
    });
    window.add_controller(focus_controller);

    window.present();

    // Generate initial password
    let update_password_for_idle = update_password.clone();
    let settings_for_idle = settings.clone();
    glib::idle_add_local_once(move || {
        update_password_for_idle(settings_for_idle.borrow().groups);
    });
}

fn strength_tier(bits: f64) -> usize {
    match bits {
        b if b < 40.0 => 0,
        b if b < 60.0 => 1,
        b if b < 80.0 => 2,
        b if b < 100.0 => 3,
        _ => 4,
    }
}

/// Expected time to find the password by brute force: on average an attacker
/// has to walk half the keyspace, so 2^(bits-1) guesses.
fn crack_time_seconds(bits: f64) -> f64 {
    (bits - 1.0).exp2() / GUESSES_PER_SECOND
}

fn superscript(mut n: u32) -> String {
    const DIGITS_SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    if n == 0 {
        return DIGITS_SUP[0].to_string();
    }
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(DIGITS_SUP[(n % 10) as usize]);
        n /= 10;
    }
    digits.reverse();
    digits.into_iter().collect()
}

fn password_entropy_bits(
    groups: i32,
    options: &GenerationOptions,
    use_default_strategy: bool,
) -> Option<f64> {
    entropy_bits_for_length((groups.max(1) * 5) as usize, options, use_default_strategy)
}

/// Exact Shannon entropy of the distribution `generate_password` draws from.
///
/// Both strategies produce a uniform distribution over their reachable
/// passwords, so the entropy is `log2(number of reachable passwords)`.
///
/// For the default strategy that is *not* `length × log2(pool)`: the base is
/// drawn from lowercase only, and one position per enabled extra character
/// class is overwritten. Counting it properly (free lowercase positions, the
/// ordered choice of positions to overwrite, and the forced characters
/// themselves) gives a noticeably lower — and honest — figure.
fn entropy_bits_for_length(
    total_chars: usize,
    options: &GenerationOptions,
    use_default_strategy: bool,
) -> Option<f64> {
    if total_chars == 0 {
        return None;
    }

    if use_default_strategy {
        let forced: Vec<usize> = [
            (options.uppercase, UPPER.len()),
            (options.digits, DIGITS.len()),
            (options.special, SPECIAL.len()),
        ]
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, size)| size)
        .collect();

        if forced.len() > total_chars {
            return None;
        }

        // Positions left untouched, each a uniform lowercase character.
        let mut bits = (total_chars - forced.len()) as f64 * (LOWER.len() as f64).log2();
        // Ordered choice of the distinct positions that get overwritten:
        // total × (total-1) × … × (total-forced+1) possibilities.
        for i in 0..forced.len() {
            bits += ((total_chars - i) as f64).log2();
        }
        // The forced characters themselves.
        for size in forced {
            bits += (size as f64).log2();
        }
        Some(bits)
    } else {
        let pool = options.pool().len();
        if pool == 0 {
            return None;
        }
        Some(total_chars as f64 * (pool as f64).log2())
    }
}

fn generate_password(groups: i32, options: &GenerationOptions, use_default_strategy: bool) -> String {
    let mut rng = OsRng;
    let total_groups = groups.max(1);
    let total_chars = (total_groups * 5) as usize;

    let mut password_chars: Vec<u8>;

    if use_default_strategy {
        password_chars = vec![0u8; total_chars];
        for ch in password_chars.iter_mut() {
            let idx = rng.gen_range(0..LOWER.len());
            *ch = LOWER[idx];
        }

        let mut forced_pools: Vec<&[u8]> = Vec::new();
        if options.uppercase {
            forced_pools.push(UPPER);
        }
        if options.digits {
            forced_pools.push(DIGITS);
        }
        if options.special {
            forced_pools.push(SPECIAL);
        }

        if forced_pools.len() > total_chars {
            return String::new();
        }

        let mut positions: Vec<usize> = (0..total_chars).collect();
        positions.shuffle(&mut rng);

        for pool in forced_pools {
            if let Some(pos) = positions.pop() {
                let idx = rng.gen_range(0..pool.len());
                password_chars[pos] = pool[idx];
            }
        }
    } else {
        let pool = options.pool();
        if pool.is_empty() {
            return String::new();
        }

        password_chars = Vec::with_capacity(total_chars);
        for _ in 0..total_chars {
            let idx = rng.gen_range(0..pool.len());
            password_chars.push(pool[idx]);
        }
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

const DC_PENDING: u8 = 0;
const DC_OK: u8 = 1;
const DC_ERR: u8 = 2;

// Copy via the Wayland data-control protocol (ext-data-control-v1 /
// wlr-data-control), the same mechanism wl-copy uses. Unlike the GTK
// clipboard this does not require keyboard focus, so it works right at
// startup before the user has interacted with the window. The spawned
// thread keeps serving paste requests until the selection is replaced;
// on GNOME the compositor caches the content immediately, so it also
// survives the app quitting.
fn spawn_data_control_copy(text: String) -> Arc<AtomicU8> {
    let state = Arc::new(AtomicU8::new(DC_PENDING));
    let state_for_thread = state.clone();
    std::thread::spawn(move || {
        use wl_clipboard_rs::copy::{MimeType, Options, Source};
        let mut options = Options::new();
        // prepare_copy requires foreground mode; we serve from this thread.
        options.foreground(true);
        let result =
            options.prepare_copy(Source::Bytes(text.into_bytes().into()), MimeType::Text);
        match result {
            Ok(prepared) => {
                state_for_thread.store(DC_OK, Ordering::SeqCst);
                if let Err(e) = prepared.serve() {
                    eprintln!("data-control serve failed: {e:?}");
                }
            }
            Err(e) => {
                eprintln!("data-control copy unavailable: {e:?}");
                state_for_thread.store(DC_ERR, Ordering::SeqCst);
            }
        }
    });
    state
}

fn window_is_active(window: &ApplicationWindow) -> bool {
    window.upcast_ref::<gtk::Window>().is_active()
}

// Shared state for one auto-copy request.
struct CopyState {
    pending: RefCell<Option<String>>,
    dc_state: RefCell<Option<Arc<AtomicU8>>>,
    gen: Cell<u64>,
    feedback_shown: Cell<bool>,
    // True while the window holds real keyboard focus (EventControllerFocus
    // enter/leave). Unlike the is-active property this only turns true after
    // GDK processed the keyboard-enter event, which is when a valid Wayland
    // input serial for clipboard writes is guaranteed to exist.
    keyboard_focus: Cell<bool>,
}

impl CopyState {
    fn new() -> Rc<Self> {
        Rc::new(CopyState {
            pending: RefCell::new(None),
            dc_state: RefCell::new(None),
            gen: Cell::new(0),
            feedback_shown: Cell::new(false),
            keyboard_focus: Cell::new(false),
        })
    }

    fn cancel(&self) {
        self.gen.set(self.gen.get().wrapping_add(1));
        self.feedback_shown.set(false);
        self.pending.borrow_mut().take();
        self.dc_state.borrow_mut().take();
    }
}

// Try to complete a pending clipboard copy.
//
// Preferred path: the Wayland data-control copy running in a background
// thread (no focus needed). Fallback: the GTK clipboard, which on Wayland
// only takes effect while the window has keyboard focus (the compositor
// validates the input-event serial), so it is gated on the window being
// active. Returns true once the copy is done.
fn try_pending_copy(
    window: &ApplicationWindow,
    copy_state: &Rc<CopyState>,
    show_feedback: &Rc<dyn Fn()>,
    strings: &Rc<I18nStrings>,
) -> bool {
    if copy_state.pending.borrow().is_none() {
        return false;
    }

    let dc = copy_state
        .dc_state
        .borrow()
        .as_ref()
        .map(|s| s.load(Ordering::SeqCst))
        .unwrap_or(DC_ERR);

    match dc {
        DC_OK => {}
        // Data-control result not in yet; the retry timer checks again.
        DC_PENDING => return false,
        // No data-control support: fall back to the GTK clipboard, which
        // needs real keyboard focus for the write to be accepted.
        _ => {
            if !copy_state.keyboard_focus.get() {
                return false;
            }
        }
    }

    let text = match copy_state.pending.borrow_mut().take() {
        Some(t) => t,
        None => return false,
    };
    if dc != DC_OK {
        copy_to_clipboard(window, &text);
    }
    println!("{}", strings.clipboard_log(&text));
    if !copy_state.feedback_shown.get() {
        show_feedback();
        copy_state.feedback_shown.set(true);
    }
    true
}

fn schedule_auto_copy(
    window: &ApplicationWindow,
    copy_state: Rc<CopyState>,
    show_feedback: Rc<dyn Fn()>,
    strings: Rc<I18nStrings>,
    text: String,
) {
    let gen = copy_state.gen.get().wrapping_add(1);
    copy_state.gen.set(gen);
    copy_state.feedback_shown.set(false);
    *copy_state.pending.borrow_mut() = Some(text.clone());
    *copy_state.dc_state.borrow_mut() = Some(spawn_data_control_copy(text));

    if try_pending_copy(window, &copy_state, &show_feedback, &strings) {
        return;
    }

    // Poll until the data-control thread reports a result or the window
    // gains focus (the notify::is-active handler also retries then). Give
    // up after 5 seconds; a later focus change can still finish the copy.
    let window_weak = window.downgrade();
    let mut attempts = 0u32;
    glib::timeout_add_local(Duration::from_millis(50), move || {
        if copy_state.gen.get() != gen {
            return glib::ControlFlow::Break;
        }
        let window = match window_weak.upgrade() {
            Some(w) => w,
            None => return glib::ControlFlow::Break,
        };
        if try_pending_copy(&window, &copy_state, &show_feedback, &strings) {
            return glib::ControlFlow::Break;
        }
        attempts += 1;
        if attempts >= 100 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
