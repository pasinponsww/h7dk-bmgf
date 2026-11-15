use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINE: usize = 160;

fn fail(message: &str) -> ! {
    panic!("\n\nstory error: {message}\n");
}

fn text<'a>(table: &'a toml::Table, key: &str) -> Option<&'a str> {
    table.get(key).and_then(toml::Value::as_str)
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

        writeln!(code, "    Chapter {{ title: {title:?}, hotspots: &[").unwrap();
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
            writeln!(
                code,
                "        hs({}, {oi}, {icon:?}, {line:?}),",
                objects.len()
            )
            .unwrap();
        }
        writeln!(code, "    ] }},").unwrap();
    }
    writeln!(code, "];").unwrap();

    fs::write(out.join("chapters.rs"), code).expect("could not write chapters.rs");
}
