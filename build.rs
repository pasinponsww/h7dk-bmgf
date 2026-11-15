use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINE: usize = 160;

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

    let mut code = String::new();
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
                "        hs({}, {oi}, {icon:?}, {line:?}, {zh:?}, {pinyin:?}),",
                objects.len()
            )
            .unwrap();
        }
        writeln!(code, "    ] }},").unwrap();
    }
    writeln!(code, "];\n").unwrap();
    code.push_str(&ending_code(&doc));

    fs::write(out.join("chapters.rs"), code).expect("could not write chapters.rs");
}
