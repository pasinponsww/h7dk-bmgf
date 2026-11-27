use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINE: usize = 160;
/// Icons are baked at this many pixels square. The object tiles are 64.
const ICON_PX: u32 = 44;

fn fail(message: &str) -> ! {
    panic!("\n\nstory error: {message}\n");
}

const MAX_LABEL: usize = 14;
const NO_LABEL_COUNT: usize = 6;

const DEFAULT_QUESTION: &str = "Will you be my girlfriend?";
const DEFAULT_YES: &str = "YES";
const DEFAULT_NO: [&str; NO_LABEL_COUNT] = [
    "No",
    "Wait, no",
    "Sure?",
    "Error 404",
    "Really?",
    "Okay, yes",
];
const DEFAULT_TITLE: &str = "You said yes";
const DEFAULT_TRACE_PROMPT: &str = "Draw a heart for me";
const DEFAULT_MESSAGE: &str = "best day ever";

fn text<'a>(table: &'a toml::Table, key: &str) -> Option<&'a str> {
    table.get(key).and_then(toml::Value::as_str)
}

fn limited<'a>(table: &'a toml::Table, key: &str, max: usize, label: &str) -> Option<&'a str> {
    let value = text(table, key)?;
    let length = value.chars().count();
    if value.trim().is_empty() {
        fail(&format!("{label}: \"{key}\" is empty"));
    }
    if length > max {
        fail(&format!(
            "{label}: \"{key}\" is {length} characters, the limit is {max}"
        ));
    }
    Some(value)
}

/// Reads the optional `zh` and `pinyin` pair. They go together: one without the other is an error.
fn chinese<'a>(table: &'a toml::Table, label: &str) -> (&'a str, &'a str) {
    let zh = limited(table, "zh", MAX_LINE, label);
    let pinyin = limited(table, "pinyin", MAX_LINE, label);
    match (zh, pinyin) {
        (Some(zh), Some(pinyin)) => (zh, pinyin),
        (None, None) => ("", ""),
        _ => fail(&format!(
            "{label}: \"zh\" and \"pinyin\" must be given together"
        )),
    }
}

fn section<'a>(doc: &'a toml::Table, key: &str) -> Option<&'a toml::Table> {
    doc.get(key).map(|v| {
        v.as_table()
            .unwrap_or_else(|| fail(&format!("[{key}] must be a table")))
    })
}

fn ending_code(doc: &toml::Table) -> String {
    let empty = toml::Table::new();
    let question = section(doc, "question").unwrap_or(&empty);
    let celebration = section(doc, "celebration").unwrap_or(&empty);

    let text_q = limited(question, "text", MAX_LINE, "[question]").unwrap_or(DEFAULT_QUESTION);
    let (zh_q, py_q) = chinese(question, "[question]");
    let yes = limited(question, "yes", 8, "[question]").unwrap_or(DEFAULT_YES);

    let mut no: Vec<String> = DEFAULT_NO.iter().map(|s| (*s).to_string()).collect();
    if let Some(value) = question.get("no_labels") {
        let list = value
            .as_array()
            .unwrap_or_else(|| fail("[question]: \"no_labels\" must be a list of texts"));
        if list.len() != NO_LABEL_COUNT {
            fail(&format!(
                "[question]: \"no_labels\" needs exactly {NO_LABEL_COUNT} texts, one per jump and the last one for when NO gives up, not {}",
                list.len()
            ));
        }
        no = list
            .iter()
            .map(|v| {
                let s = v
                    .as_str()
                    .unwrap_or_else(|| fail("[question]: \"no_labels\" must hold only texts"));
                if s.trim().is_empty() || s.chars().count() > MAX_LABEL {
                    fail(&format!(
                        "[question]: the no label \"{s}\" must be 1 to {MAX_LABEL} characters"
                    ));
                }
                s.to_string()
            })
            .collect();
    }

    let title = limited(celebration, "title", 40, "[celebration]").unwrap_or(DEFAULT_TITLE);
    let message =
        limited(celebration, "message", MAX_LINE, "[celebration]").unwrap_or(DEFAULT_MESSAGE);
    let (zh_c, py_c) = chinese(celebration, "[celebration]");

    let labels = no
        .iter()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "/// The question screen and the ending.\npub static ENDING: Ending = Ending {{\n    question: Phrase {{ en: {text_q:?}, zh: {zh_q:?}, pinyin: {py_q:?} }},\n    yes: {yes:?},\n    no_labels: [{labels}],\n    title: {title:?},\n    message: Phrase {{ en: {message:?}, zh: {zh_c:?}, pinyin: {py_c:?} }},\n}};\n"
    )
}

/// The most bytes a secret may have (as in the app's `secret.rs`).
const MAX_SECRET: usize = 90;

/// FNV-1a over `bytes`, as 64 bits.
fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// `[celebration] secret`, sealed: "LOVE" and the text encrypted with
/// AES-128 in counter mode (see the app's `secret.rs`). The key and the
/// first counter block come from the text itself, so a build is the same
/// every time; the last 32 bits of the counter start at zero, as the chip's
/// AES engine counts them.
fn secret_code(doc: &toml::Table) -> String {
    use aes::cipher::{KeyIvInit, StreamCipher};
    let empty = toml::Table::new();
    let celebration = section(doc, "celebration").unwrap_or(&empty);
    let Some(text) = limited(celebration, "secret", MAX_SECRET, "[celebration]") else {
        return "/// No secret message.\npub static SECRET: Option<crate::secret::Sealed> = None;\n"
            .to_string();
    };
    if text.len() > MAX_SECRET {
        fail(&format!(
            "[celebration]: \"secret\" is {} bytes, the limit is {MAX_SECRET}",
            text.len()
        ));
    }
    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    for (i, chunk) in key.chunks_mut(8).enumerate() {
        let h = fnv64(format!("key{i}:{text}").as_bytes());
        chunk.copy_from_slice(&h.to_le_bytes());
    }
    let h = fnv64(format!("iv:{text}").as_bytes());
    iv[..8].copy_from_slice(&h.to_le_bytes());
    iv[8..12].copy_from_slice(&h.to_be_bytes()[..4]);
    let mut bytes = b"LOVE".to_vec();
    bytes.extend_from_slice(text.as_bytes());
    let mut c = ctr::Ctr32BE::<aes::Aes128>::new(&key.into(), &iv.into());
    c.apply_keystream(&mut bytes);
    format!(
        "/// The secret message, sealed.\npub static SECRET: Option<crate::secret::Sealed> = Some(crate::secret::Sealed {{ key: {key:?}, iv: {iv:?}, bytes: &{bytes:?} }});\n"
    )
}

/// The places a moment can happen in (as `ui::places::Place::NAMES`).
const PLACES: [&str; 10] = [
    "pink", "club", "beach", "fall", "cinema", "city", "museum", "night", "cafe", "dream",
];
/// The acts the dogs can play (as `app::moment::Act::NAMES`).
const ACTS: [&str; 22] = [
    "wander",
    "wow",
    "smitten",
    "think",
    "excited",
    "heartbeat",
    "laugh",
    "grin",
    "summer",
    "chill",
    "gaze",
    "movie",
    "scared",
    "gelato",
    "treats",
    "museum",
    "stroll",
    "hug",
    "ramen",
    "matcha",
    "zoomies",
    "love",
];

/// A name from `names` as its Rust variant: "club" is `Club`.
fn variant(name: &str) -> String {
    let mut c = name.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

/// Checks `name` is one of `names` (a `what` in `label`).
fn one_of<'a>(name: &'a str, names: &[&str], what: &str, label: &str) -> &'a str {
    if !names.contains(&name) {
        fail(&format!(
            "{label}: there is no {what} called \"{name}\". The {what}s are: {}",
            names.join(", ")
        ));
    }
    name
}

/// The moment for an object: its `moment` table's place (the chapter's if
/// left out) and act (wandering if left out), as Rust code.
fn moment_code(object: &toml::Table, chapter_place: &str, label: &str) -> String {
    let empty = toml::Table::new();
    let m = match object.get("moment") {
        None => &empty,
        Some(v) => v
            .as_table()
            .unwrap_or_else(|| fail(&format!("{label}: \"moment\" must be a table"))),
    };
    let place = text(m, "place").map_or(chapter_place, |p| one_of(p, &PLACES, "place", label));
    let act = text(m, "act").map_or("wander", |a| one_of(a, &ACTS, "act", label));
    format!(
        "crate::moment::Moment {{ place: ui::places::Place::{}, act: crate::moment::Act::{} }}",
        variant(place),
        variant(act)
    )
}

/// The props a finale slide may float about, and their Rust variants (as
/// `app::moment::Prop`).
const PROPS: [(&str, &str); 13] = [
    ("thought", "Thought"),
    ("heart", "Heart"),
    ("laptop", "Laptop"),
    ("sun", "Sun"),
    ("popcorn", "Popcorn"),
    ("gelato", "Gelato"),
    ("boba", "Boba"),
    ("sushi", "Sushi"),
    ("ramen", "Ramen"),
    ("phone", "Phone"),
    ("matcha", "Matcha"),
    ("note", "Note"),
    ("sparkle", "Sparkle"),
];
/// The most props one finale slide may float about.
const MAX_SLIDE_PROPS: usize = 6;

/// Finale photos are baked at this many pixels square (50 KB of flash each).
const FINALE_PX: u32 = 160;
/// The most different photos the finale may use: they live in the chip's
/// own flash, which holds about this many next to the firmware.
const MAX_FINALE_PHOTOS: usize = 14;
/// The most slides the finale may have.
const MAX_SLIDES: usize = 40;

/// The `[[finale]]` slides: sentences, some with a photo, that play between
/// the drawn heart and the question. A photo used on several slides is
/// baked once.
fn finale_code(doc: &toml::Table, dir: &Path, out: &Path) -> String {
    let slides: &[toml::Value] = doc
        .get("finale")
        .and_then(toml::Value::as_array)
        .map_or(&[], Vec::as_slice);
    if slides.len() > MAX_SLIDES {
        fail(&format!(
            "the finale has {} slides, the limit is {MAX_SLIDES}",
            slides.len()
        ));
    }
    let mut photos: BTreeMap<String, String> = BTreeMap::new();
    let mut statics = String::new();
    let mut rows = String::new();
    for (i, slide) in slides.iter().enumerate() {
        let label = format!("finale slide {}", i + 1);
        let table = slide
            .as_table()
            .unwrap_or_else(|| fail(&format!("{label} is not a table")));
        let line = limited(table, "line", MAX_LINE, &label)
            .unwrap_or_else(|| fail(&format!("{label} needs a line")));
        let photo = match text(table, "photo") {
            Some(file) => {
                if !photos.contains_key(file) {
                    if photos.len() == MAX_FINALE_PHOTOS {
                        fail(&format!(
                            "{label}: the finale may use {MAX_FINALE_PHOTOS} different photos at most (they live in the chip's flash)"
                        ));
                    }
                    let ident = format!("FINALE_PHOTO_{}", photos.len() + 1);
                    statics.push_str(&bake_photo_at(dir, out, file, &label, &ident, FINALE_PX));
                    photos.insert(file.to_string(), ident);
                }
                format!("Some(&{})", photos[file])
            }
            None => "None".to_string(),
        };
        let place = text(table, "place").map_or("dream", |p| one_of(p, &PLACES, "place", &label));
        let props: Vec<String> = table
            .get("props")
            .map(|v| {
                v.as_array()
                    .unwrap_or_else(|| fail(&format!("{label}: \"props\" must be a list")))
                    .iter()
                    .map(|p| {
                        let name = p.as_str().unwrap_or("");
                        let found = PROPS.iter().find(|(n, _)| *n == name);
                        let Some((_, variant)) = found else {
                            let names: Vec<&str> = PROPS.iter().map(|(n, _)| *n).collect();
                            fail(&format!(
                                "{label}: there is no prop called \"{name}\". The props are: {}",
                                names.join(", ")
                            ));
                        };
                        format!("crate::moment::Prop::{variant}")
                    })
                    .collect()
            })
            .unwrap_or_default();
        if props.len() > MAX_SLIDE_PROPS {
            fail(&format!(
                "{label}: {} props, a slide may have {MAX_SLIDE_PROPS}",
                props.len()
            ));
        }
        writeln!(
            rows,
            "    Slide {{ line: {line:?}, photo: {photo}, place: ui::places::Place::{}, props: &[{}] }},",
            variant(place),
            props.join(", ")
        )
        .unwrap();
    }
    format!(
        "{statics}/// The finale: slides between the drawn heart and the question.\npub static FINALE: [Slide; {}] = [\n{rows}];\n",
        slides.len()
    )
}

/// The optional `[trace]` table: the trace-a-heart game before the question.
fn trace_code(doc: &toml::Table) -> String {
    let empty = toml::Table::new();
    let trace = section(doc, "trace").unwrap_or(&empty);
    let enabled = trace.get("enabled").is_none_or(|v| {
        v.as_bool()
            .unwrap_or_else(|| fail("[trace]: \"enabled\" must be true or false"))
    });
    let prompt = limited(trace, "prompt", 40, "[trace]").unwrap_or(DEFAULT_TRACE_PROMPT);
    format!(
        "/// The trace-a-heart game before the question.
pub static TRACE: Trace = Trace {{ enabled: {enabled}, prompt: {prompt:?} }};
"
    )
}

/// `(year, month, day)` from "YYYY-MM-DD", if it is a real date.
fn parse_date(s: &str) -> Option<(u16, u8, u8)> {
    let mut parts = s.trim().splitn(3, '-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };
    (1..=days).contains(&day).then_some((year, month, day))
}

/// `(year, month, day)`, `hour`, `minute`, `second`.
type DateTimeParts = ((u16, u8, u8), u8, u8, u8);

/// The parts of "YYYY-MM-DDTHH:MM:SS".
fn parse_date_time(s: &str) -> Option<DateTimeParts> {
    let (date, time) = s.trim().split_once('T')?;
    let mut t = time.splitn(3, ':');
    let hour: u8 = t.next()?.parse().ok()?;
    let minute: u8 = t.next()?.parse().ok()?;
    let second: u8 = t.next()?.get(..2)?.parse().ok()?;
    (hour < 24 && minute < 60 && second < 60).then_some((parse_date(date)?, hour, minute, second))
}

/// `[together] since` from the story, and the PC's clock when the firmware
/// was built (`H7_NOW`, set by `cargo proj mbedded ...`), for `calendar.rs`.
fn write_clock(doc: &toml::Table, out: &Path) {
    println!("cargo:rerun-if-env-changed=H7_NOW");
    let since = section(doc, "together")
        .and_then(|t| text(t, "since"))
        .map(|s| {
            parse_date(s).unwrap_or_else(|| {
                fail(&format!(
                    "[together]: \"since\" is \"{s}\", it must be a real date like 2024-02-14"
                ))
            })
        });
    let since = since.map_or("None".to_string(), |(y, m, d)| {
        format!("Some(Date {{ year: {y}, month: {m}, day: {d} }})")
    });
    let now = env::var("H7_NOW").ok().and_then(|s| parse_date_time(&s));
    let now = now.map_or("None".to_string(), |((y, mo, d), h, mi, s)| {
        format!(
            "Some(DateTime {{ date: Date {{ year: {y}, month: {mo}, day: {d} }}, hour: {h}, minute: {mi}, second: {s} }})"
        )
    });
    let code = format!(
        "/// When it all began (`[together] since` in the story).\n\
         pub static TOGETHER_SINCE: Option<Date> = {since};\n\
         /// The PC's local time when the firmware was built.\n\
         pub static BUILT_AT: Option<DateTime> = {now};\n"
    );
    fs::write(out.join("clock.rs"), code).expect("could not write clock.rs");
}

/// One pixel font to bake from a TTF file.
struct FontJob {
    /// The Rust name of the font.
    name: &'static str,
    /// The TTF file, inside `assets/fonts/pixel`.
    file: &'static str,
    /// Size to render at, in pixels.
    px: f32,
    /// Width and height of one character cell.
    cell: (u32, u32),
    /// Coverage (0 to 255) a pixel needs to be drawn.
    threshold: u8,
}

/// Bakes `job` into an embedded-graphics `MonoFont`: the 96 glyphs from space
/// to DEL in a 16 by 6 grid of cells, one bit per pixel, written to
/// `out/fonts/<name>.raw`. Returns the Rust code of the font.
fn bake_font(root: &Path, out: &Path, job: &FontJob) -> String {
    let file = root.join("assets/fonts/pixel").join(job.file);
    println!("cargo:rerun-if-changed={}", file.display());
    let data =
        fs::read(&file).unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", file.display())));
    let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
        .unwrap_or_else(|e| fail(&format!("{}: {e}", file.display())));
    let ascent = font
        .horizontal_line_metrics(job.px)
        .map_or(job.px, |m| m.ascent)
        .round() as i32;
    let (cw, ch) = job.cell;
    let cols = 16u32;
    let width = cols * cw;
    let stride = width.div_ceil(8);
    let mut raw = vec![0u8; (stride * 6 * ch) as usize];
    for (i, code) in (0x20u8..=0x7f).enumerate() {
        if code == 0x7f {
            continue;
        }
        let (m, bitmap) = font.rasterize(char::from(code), job.px);
        let (cx, cy) = (i as u32 % cols * cw, i as u32 / cols * ch);
        // fontdue's ymin is how far the bitmap's bottom sits above the baseline.
        let top = ascent - (m.ymin + m.height as i32);
        for y in 0..m.height {
            for x in 0..m.width {
                if bitmap[y * m.width + x] < job.threshold {
                    continue;
                }
                let gx = m.xmin + x as i32;
                let gy = top + y as i32;
                if gx < 0 || gy < 0 || gx >= cw as i32 || gy >= ch as i32 {
                    fail(&format!(
                        "{}: '{}' does not fit its {cw} x {ch} cell",
                        job.file,
                        char::from(code)
                    ));
                }
                let (px, py) = (cx + gx as u32, cy + gy as u32);
                raw[(py * stride + px / 8) as usize] |= 0x80 >> (px % 8);
            }
        }
    }
    let dir = out.join("fonts");
    fs::create_dir_all(&dir).expect("could not create the font folder");
    let path = dir.join(format!("{}.raw", job.name.to_lowercase()));
    fs::write(&path, raw).expect("could not write a font");
    let path = path.display().to_string();
    let baseline = (ascent - 1).max(0);
    format!(
        "/// {file} at {px} px, baked by build.rs.
         pub const {name}: MonoFont = MonoFont {{
             image: ImageRaw::new(include_bytes!({path:?}), {width}),
             glyph_mapping: &mapping::ASCII,
             character_size: Size::new({cw}, {ch}),
             character_spacing: 0,
             baseline: {baseline},
             underline: DecorationDimensions::new({under}, 1),
             strikethrough: DecorationDimensions::new({strike}, 1),
         }};
",
        file = job.file,
        px = job.px,
        name = job.name,
        under = baseline + 2,
        strike = ch / 2,
    )
}

/// The pixel fonts in `assets/fonts/pixel`, baked for embedded-graphics.
fn write_fonts(root: &Path, out: &Path) {
    let jobs = [
        // Drawn on an 8 x 8 grid: crisp at 8 px, scaled up on the board.
        FontJob {
            name: "PRESS_START",
            file: "PressStart2P-Regular.ttf",
            px: 8.0,
            cell: (8, 8),
            threshold: 128,
        },
        FontJob {
            name: "VT323",
            file: "VT323-Regular.ttf",
            px: 20.0,
            cell: (8, 20),
            threshold: 128,
        },
    ];
    let code: String = jobs.iter().map(|j| bake_font(root, out, j)).collect();
    fs::write(out.join("fonts.rs"), code).expect("could not write fonts.rs");
}

fn main() {
    let root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    println!("cargo:rerun-if-changed=build.rs");

    write_story_table(&root, &out);
    write_fonts(&root, &out);

    if env::var_os("CARGO_FEATURE_FIRMWARE").is_some() {
        link_memory_map(&root, &out);
    }
}

fn link_memory_map(root: &Path, out: &Path) {
    fs::copy(root.join("memory.x"), out.join("memory.x")).expect("could not copy memory.x");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
}

/// Splits a Phosphor duotone SVG into its soft fill layer (the paths drawn at
/// 20% opacity) and its outline layer, as two SVGs drawn fully opaque.
fn icon_layers(svg: &str, file: &Path) -> (String, String) {
    let bad = |why: &str| -> ! { fail(&format!("{}: {why}", file.display())) };
    let head_end = svg
        .find("<svg")
        .and_then(|s| svg[s..].find('>').map(|e| s + e))
        .unwrap_or_else(|| bad("no <svg> tag"));
    let head = &svg[..=head_end];
    let (mut fill, mut line) = (String::new(), String::new());
    let mut rest = &svg[head_end + 1..];
    while let Some(start) = rest.find("<path") {
        let end = rest[start..]
            .find("/>")
            .map(|e| start + e + 2)
            .unwrap_or_else(|| bad("a <path> is not closed with />"));
        let element = &rest[start..end];
        match element.find(" opacity=\"") {
            Some(at) => {
                let value_end = element[at + 10..]
                    .find('"')
                    .map(|e| at + 10 + e + 1)
                    .unwrap_or_else(|| bad("an opacity value is not closed"));
                fill.push_str(&element[..at]);
                fill.push_str(&element[value_end..]);
            }
            None => line.push_str(element),
        }
        rest = &rest[end..];
    }
    if line.is_empty() {
        bad("no outline <path> found");
    }
    (format!("{head}{fill}</svg>"), format!("{head}{line}</svg>"))
}

/// How much `svg` covers each pixel when drawn `size` pixels square, from 0
/// to 15, row by row.
fn coverage(svg: &str, size: u32, file: &Path) -> Vec<u8> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .unwrap_or_else(|e| fail(&format!("{}: {e}", file.display())));
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("icon size is not zero");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .pixels()
        .iter()
        .map(|p| u8::try_from((u32::from(p.alpha()) * 15 + 127) / 255).unwrap_or(15))
        .collect()
}

/// Bakes every icon the story uses into `out/icons/<name>.bin` (see
/// `ui::icon`) and returns the Rust statics that hold them, keyed by icon name.
fn bake_icons<'a>(
    icons: &Path,
    out: &Path,
    names: impl Iterator<Item = &'a str>,
) -> BTreeMap<String, String> {
    let dir = out.join("icons");
    fs::create_dir_all(&dir).expect("could not create the icon folder");
    let mut statics = BTreeMap::new();
    for name in names {
        if statics.contains_key(name) {
            continue;
        }
        let file = icons.join(format!("{name}.svg"));
        println!("cargo:rerun-if-changed={}", file.display());
        let svg = fs::read_to_string(&file)
            .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", file.display())));
        let (fill, line) = icon_layers(&svg, &file);
        let fill = coverage(&fill, ICON_PX, &file);
        let line = coverage(&line, ICON_PX, &file);
        let px: Vec<u8> = line.iter().zip(&fill).map(|(l, f)| l << 4 | f).collect();
        let bin = dir.join(format!("{name}.bin"));
        fs::write(&bin, px).expect("could not write an icon");
        let ident = icon_ident(name);
        let bin = bin.display().to_string();
        statics.insert(
            name.to_string(),
            format!(
                "static {ident}: ui::icon::Icon = ui::icon::Icon::new({ICON_PX}, include_bytes!({bin:?}));
"
            ),
        );
    }
    statics
}

fn icon_ident(name: &str) -> String {
    format!("ICON_{}", name.to_uppercase().replace('-', "_"))
}

/// Photos are baked at this many pixels square (about 32 KB of flash each).
const PHOTO_PX: u32 = 128;

/// Floyd-Steinberg dithering from 8 bits per channel to RGB565: the rounding
/// error of each pixel is pushed onto its neighbors, so gradients (skin, sky)
/// stay smooth instead of banding.
fn dither_rgb565(rgb: &[[u8; 3]], w: usize, h: usize) -> Vec<u16> {
    let mut work: Vec<[f32; 3]> = rgb
        .iter()
        .map(|p| [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])])
        .collect();
    let mut out = vec![0u16; w * h];
    let levels = [31.0f32, 63.0, 31.0];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut q = [0u16; 3];
            let mut err = [0f32; 3];
            for c in 0..3 {
                let v = work[i][c].clamp(0.0, 255.0);
                let n = (v * levels[c] / 255.0).round();
                q[c] = n as u16;
                err[c] = v - n * 255.0 / levels[c];
            }
            out[i] = q[0] << 11 | q[1] << 5 | q[2];
            let mut push = |dx: isize, dy: usize, share: f32| {
                let nx = x as isize + dx;
                if nx < 0 || nx >= w as isize || y + dy >= h {
                    return;
                }
                let j = (y + dy) * w + nx as usize;
                for c in 0..3 {
                    work[j][c] += err[c] * share;
                }
            };
            push(1, 0, 7.0 / 16.0);
            push(-1, 1, 3.0 / 16.0);
            push(0, 1, 5.0 / 16.0);
            push(1, 1, 1.0 / 16.0);
        }
    }
    out
}

/// Bakes the photo `file` (in the story folder) into `out/photos/<ident>.bin`:
/// cut to a square (the middle of a wide photo, nearer the top of a tall
/// one), shrunk to [`PHOTO_PX`] and dithered to RGB565,
/// low byte first. Returns the Rust static that holds it.
fn bake_photo(story_dir: &Path, out: &Path, file: &str, label: &str, ident: &str) -> String {
    bake_photo_at(story_dir, out, file, label, ident, PHOTO_PX)
}

/// [`bake_photo`] at `side` pixels square.
fn bake_photo_at(
    story_dir: &Path,
    out: &Path,
    file: &str,
    label: &str,
    ident: &str,
    side: u32,
) -> String {
    let path = story_dir.join(file);
    println!("cargo:rerun-if-changed={}", path.display());
    let img = image::open(&path)
        .unwrap_or_else(|e| {
            fail(&format!(
                "{label}: cannot open the photo {}: {e}",
                path.display()
            ))
        })
        .to_rgb8();
    let (w, h) = img.dimensions();
    // A wide photo keeps its middle; a tall one keeps nearer its top, where
    // faces usually are (a fifth of the spare height above, not half).
    let crop = w.min(h);
    let square =
        image::imageops::crop_imm(&img, (w - crop) / 2, (h - crop) / 5, crop, crop).to_image();
    let small = image::imageops::resize(&square, side, side, image::imageops::FilterType::Lanczos3);
    let rgb: Vec<[u8; 3]> = small.pixels().map(|p| p.0).collect();
    let px = dither_rgb565(&rgb, side as usize, side as usize);
    let bytes: Vec<u8> = px.iter().flat_map(|v| v.to_le_bytes()).collect();
    let dir = out.join("photos");
    fs::create_dir_all(&dir).expect("could not create the photo folder");
    let bin = dir.join(format!("{}.bin", ident.to_lowercase()));
    fs::write(&bin, bytes).expect("could not write a photo");
    let bin = bin.display().to_string();
    format!(
        "static {ident}: ui::photo::Photo = ui::photo::Photo::new({side}, include_bytes!({bin:?}));\n"
    )
}

fn write_story_table(root: &Path, out: &Path) {
    println!("cargo:rerun-if-env-changed=H7_STORY");
    let name = env::var("H7_STORY").unwrap_or_else(|_| "default".to_string());
    let dir = if Path::new(&name).is_absolute() {
        PathBuf::from(&name)
    } else {
        root.join("story").join(&name)
    };
    let file = dir.join("story.toml");
    println!("cargo:rerun-if-changed={}", file.display());

    let source = fs::read_to_string(&file).unwrap_or_else(|e| {
        fail(&format!(
            "cannot read {}: {e}\n  Use --story <folder> with a folder inside story/",
            file.display()
        ))
    });
    let doc: toml::Table = source
        .parse()
        .unwrap_or_else(|e| fail(&format!("{} is not valid TOML: {e}", file.display())));
    let icons = root.join("assets/icons/phosphor/duotone");

    let chapters = doc
        .get("chapter")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| fail("no [[chapter]] found"));
    if chapters.is_empty() {
        fail("the story needs at least one chapter");
    }

    let used = chapters
        .iter()
        .filter_map(|c| c.get("object").and_then(toml::Value::as_array))
        .flatten()
        .filter_map(|o| o.get("icon").and_then(toml::Value::as_str))
        .filter(|name| icons.join(format!("{name}.svg")).is_file());
    let baked = bake_icons(&icons, out, used);

    let mut photos = String::new();
    let mut code = String::new();
    for line in baked.values() {
        code.push_str(line);
    }
    writeln!(
        code,
        "/// All chapters, in order.\npub static CHAPTERS: [Chapter; {}] = [",
        chapters.len()
    )
    .unwrap();
    for (ci, chapter) in chapters.iter().enumerate() {
        let label = format!("chapter {}", ci + 1);
        let table = chapter
            .as_table()
            .unwrap_or_else(|| fail(&format!("{label} is not a table")));
        let title = text(table, "title")
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| fail(&format!("{label} needs a title")));
        let objects = table
            .get("object")
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| fail(&format!("{label} (\"{title}\") has no [[chapter.object]]")));
        if !(2..=4).contains(&objects.len()) {
            fail(&format!(
                "{label} (\"{title}\") has {} objects, it needs 2 to 4",
                objects.len()
            ));
        }

        let mood = limited(table, "mood", 12, &label).unwrap_or("");
        let place = text(table, "place").map_or("pink", |p| one_of(p, &PLACES, "place", &label));
        let idle = text(table, "act").map_or("wander", |a| one_of(a, &ACTS, "act", &label));
        let caption = limited(table, "caption", 16, &label).unwrap_or("");
        let photo = match text(table, "photo") {
            Some(file) => {
                let ident = format!("PHOTO_{}", ci + 1);
                photos.push_str(&bake_photo(&dir, out, file, &label, &ident));
                format!("Some(&{ident})")
            }
            None => "None".to_string(),
        };

        writeln!(
            code,
            "    Chapter {{ title: {title:?}, mood: {mood:?}, place: ui::places::Place::{}, act: crate::moment::Act::{}, photo: {photo}, caption: {caption:?}, hotspots: &[",
            variant(place),
            variant(idle)
        )
        .unwrap();
        for (oi, object) in objects.iter().enumerate() {
            let label = format!("{label} (\"{title}\") object {}", oi + 1);
            let table = object
                .as_table()
                .unwrap_or_else(|| fail(&format!("{label} is not a table")));
            let icon =
                text(table, "icon").unwrap_or_else(|| fail(&format!("{label} needs an icon")));
            let line = text(table, "line")
                .filter(|l| !l.trim().is_empty())
                .unwrap_or_else(|| fail(&format!("{label} needs a line")));
            let length = line.chars().count();
            if length > MAX_LINE {
                fail(&format!(
                    "{label}: the line is {length} characters, the limit is {MAX_LINE}"
                ));
            }
            if !icons.join(format!("{icon}.svg")).is_file() {
                fail(&format!(
                    "{label}: there is no icon called \"{icon}\" in assets/icons/phosphor/duotone"
                ));
            }
            let (zh, pinyin) = chinese(table, &label);
            writeln!(
                code,
                "        hs({}, {oi}, {icon:?}, &{}, {line:?}, {zh:?}, {pinyin:?}, {}),",
                objects.len(),
                icon_ident(icon),
                moment_code(table, place, &label),
            )
            .unwrap();
        }
        writeln!(code, "    ] }},").unwrap();
    }
    writeln!(code, "];\n").unwrap();
    code.push_str(&photos);
    code.push_str(&finale_code(&doc, &dir, out));
    code.push_str(&ending_code(&doc));
    code.push_str(&trace_code(&doc));
    code.push_str(&secret_code(&doc));

    fs::write(out.join("chapters.rs"), code).expect("could not write chapters.rs");
    write_clock(&doc, out);
}
