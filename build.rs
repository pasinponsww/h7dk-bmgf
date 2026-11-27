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

fn main() {
    let root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    println!("cargo:rerun-if-changed=build.rs");

    write_story_table(&root, &out);

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

        writeln!(
            code,
            "    Chapter {{ title: {title:?}, mood: {mood:?}, hotspots: &["
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
                "        hs({}, {oi}, {icon:?}, &{}, {line:?}, {zh:?}, {pinyin:?}),",
                objects.len(),
                icon_ident(icon),
            )
            .unwrap();
        }
        writeln!(code, "    ] }},").unwrap();
    }
    writeln!(code, "];\n").unwrap();
    code.push_str(&ending_code(&doc));

    fs::write(out.join("chapters.rs"), code).expect("could not write chapters.rs");
}
