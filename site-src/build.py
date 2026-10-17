#!/usr/bin/env python3
"""Build the localized website pages.

Reads site-src/template.html and site-src/i18n/<lang>.json and writes
web/index.html (English, the default) and web/<lang>/index.html for every
other language. Values in the JSON files are HTML. Run from anywhere:

    python3 site-src/build.py
"""

import html
import json
import re
from pathlib import Path

SRC = Path(__file__).resolve().parent
WEB = SRC.parent / "web"
BASE_URL = "https://passwort.dumke.me/"

# Order of the language switcher; English is the default at the site root.
LANGUAGES = [
    ("en", "English"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("es", "Español"),
    ("it", "Italiano"),
    ("sv", "Svenska"),
    ("uk", "Українська"),
    ("ja", "日本語"),
]
DEFAULT = "en"

# Inline in the root page only: send visitors to their browser language
# unless they picked a language in the switcher (remembered in site.js).
DETECT = """<script>
(function () {
  var supported = %s, choice = null;
  try { choice = localStorage.getItem("lang"); } catch (e) {}
  if (!choice) {
    var prefs = navigator.languages || [navigator.language || ""];
    for (var i = 0; i < prefs.length && !choice; i++) {
      var code = String(prefs[i]).toLowerCase().split("-")[0];
      if (supported.indexOf(code) >= 0) choice = code;
    }
  }
  if (choice && choice !== "en" && supported.indexOf(choice) >= 0)
    location.replace(choice + "/" + location.hash);
})();
</script>
"""


def page_dir(code):
    return "" if code == DEFAULT else code + "/"


def french_spacing(text):
    # Narrow no-break space before high punctuation, no-break space inside
    # guillemets; skip HTML tags so attributes stay untouched.
    def fix(chunk):
        chunk = re.sub(r" ([:;?!»])", " \\1", chunk)
        return chunk.replace("« ", "« ")

    return "".join(
        part if part.startswith("<") else fix(part)
        for part in re.split(r"(<[^>]*>)", text)
    )


def build(code, strings, template, security_txt):
    root = "" if code == DEFAULT else "../"
    if code == "fr":
        strings = {k: french_spacing(v) for k, v in strings.items()}

    links = []
    for other, name in LANGUAGES:
        href = (root + page_dir(other)) or "./"
        current = ' aria-current="page"' if other == code else ""
        links.append(
            f'    <a href="{href}" hreflang="{other}" lang="{other}" '
            f'data-lang="{other}"{current}>{name}</a>'
        )
    alternates = [
        f'<link rel="alternate" hreflang="{other}" href="{BASE_URL}{page_dir(other)}">'
        for other, _ in LANGUAGES
    ]
    alternates.append(f'<link rel="alternate" hreflang="x-default" href="{BASE_URL}">')

    values = dict(strings)
    values.update(
        lang=code,
        lang_code=code.upper(),
        root=root,
        home="./",
        url=BASE_URL + page_dir(code),
        alternates="\n".join(alternates),
        lang_links="\n".join(links),
        security_txt=html.escape(security_txt),
        detect=DETECT % json.dumps([c for c, _ in LANGUAGES])
        if code == DEFAULT
        else "",
    )

    def substitute(match):
        key = match.group(1)
        if key not in values:
            raise KeyError(f"{code}: missing string '{key}'")
        return values[key]

    out = re.sub(r"\{\{(\w+)\}\}", substitute, template)
    target = WEB / page_dir(code) / "index.html"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(out, encoding="utf-8")
    return target


def main():
    template = (SRC / "template.html").read_text(encoding="utf-8")
    security_txt = (WEB / ".well-known" / "security.txt").read_text(encoding="utf-8")
    reference = None
    for code, _ in LANGUAGES:
        strings = json.loads((SRC / "i18n" / f"{code}.json").read_text(encoding="utf-8"))
        keys = set(strings)
        if reference is None:
            reference = keys
        elif keys != reference:
            raise SystemExit(
                f"{code}: keys differ from {DEFAULT}: "
                f"missing {sorted(reference - keys)}, extra {sorted(keys - reference)}"
            )
        print(build(code, strings, template, security_txt).relative_to(WEB.parent))


if __name__ == "__main__":
    main()
