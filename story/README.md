# Stories

Each folder here holds one `story.toml`. `default/` is a placeholder that ships
with the repo. Pick another folder with `--story <folder>`, for example
`cargo proj common test --story mine`. Check one with `cargo proj story check <folder>`.

Folders other than `default/` are yours to keep private: add them to `.gitignore`.

## Format

```toml
[[chapter]]
title = "The beginning"      # big title of the page
mood = "wow"                 # optional: how the mascot dog feels on this page
photo = "us.jpg"             # optional: a jpg or png in this folder, shown as a
                             # polaroid once every object is found (32 KB of flash)
caption = "summer 2025"      # optional: up to 16 characters under the photo

[[chapter.object]]           # 2 to 4 tappable objects per chapter
icon = "door"                # a file name in assets/icons/phosphor/duotone
line = "Up to 160 characters, revealed when the object is tapped."
zh = "Optional Chinese version of the line"
pinyin = "Required if zh is given, and only then"

[together]                   # optional
since = "2024-02-14"         # the intro shows "day 652 of us", and on the day,
                             # "happy anniversary"

[question]                   # the whole table is optional
text = "Will you be my girlfriend?"
zh = "optional"
pinyin = "optional, together with zh"
yes = "YES"                  # up to 8 characters
no_labels = ["No", "Wait, no", "Sure?", "Error 404", "Really?", "Okay, yes"]

[trace]                      # the whole table is optional
enabled = true               # draw a heart before the question (false skips it)
prompt = "Draw a heart for me"   # up to 40 characters

[celebration]                # the whole table is optional
title = "You said yes"
message = "best day ever"
zh = "optional"
pinyin = "optional, together with zh"
```

`no_labels` has exactly six entries (up to 14 characters each): the NO button
shows the first one, then the next after each jump, and the last one once it
gives up.

Moods: `idle`, `happy`, `wow`, `shy`, `love`, `scared`, `cheer`. Leave `mood` out
for `idle`.

Chinese text needs a font that has those characters. The browser simulator uses
the system font; the board needs a small font subset first.
