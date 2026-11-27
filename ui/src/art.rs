//! The pixel art of the walking dogs and the props, one character per
//! pixel (see `crate::theme::sprite_rgb` for the colors, `.` is clear).
//! Drawn as ASCII so it can be edited in place.

use crate::sprite::{Art, Piece};

/// A dog facing the camera, standing (26 x 30, 6 rows of headroom).
pub const DOG_FRONT: Art = Art {
    w: 26,
    h: 30,
    rows: &[
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "...KK................KK...",
        "..KPQK..............KQPK..",
        "..KPPQK............KQPPK..",
        "..KPPPQK..........KQPPPK..",
        "..KPPPPQKKKKKKKKKKQPPPPK..",
        "..KQPPWWWWWWWWWWWWWWPPQK..",
        "..KWQWWWWWWWWWWWWWWWWQWK..",
        ".KWWWWWWWWWWWWWWWWWWWWWWK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KSWWWWWWWWWWWWWWWWWWWSSK.",
        "..KSWWWWWWWWWWWWWWWWWSSK..",
        "...KSSWWWWWWWWWWWWWWSSK...",
        "....KKSSSSSSSSSSSSSSKK....",
        ".....KHHHHHHHHHHHHHHK.....",
        "....KWWWWWWWDDWWWWWWSK....",
        "....KWWWWWWWWWWWWWWWSK....",
        "...KWWSWWWWWWWWWWWWSWSK...",
        "...KWWSKWWWWWWWWWWKSWSK...",
        "...KWWSKWWWWKKWWWWKSWSK...",
        "...KSSSKSSSSK.KSSSKSSSK...",
        "...KKKKKKKKK...KKKKKKKK...",
    ],
};

/// Facing the camera, mid-step.
pub const DOG_FRONT_STEP: Art = Art {
    w: 26,
    h: 30,
    rows: &[
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "...KK................KK...",
        "..KPQK..............KQPK..",
        "..KPPQK............KQPPK..",
        "..KPPPQK..........KQPPPK..",
        "..KPPPPQKKKKKKKKKKQPPPPK..",
        "..KQPPWWWWWWWWWWWWWWPPQK..",
        "..KWQWWWWWWWWWWWWWWWWQWK..",
        ".KWWWWWWWWWWWWWWWWWWWWWWK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KSWWWWWWWWWWWWWWWWWWWSSK.",
        "..KSWWWWWWWWWWWWWWWWWSSK..",
        "...KSSWWWWWWWWWWWWWWSSK...",
        "....KKSSSSSSSSSSSSSSKK....",
        ".....KHHHHHHHHHHHHHHK.....",
        "....KWWWWWWWDDWWWWWWSK....",
        "....KWWWWWWWWWWWWWWWSK....",
        "...KWWSWWWWWWWWWWWWSWSK...",
        "...KWWSKWWWWWWWWWWKSWSK...",
        "...KSSSKWWWWKKWWWWKSSSK...",
        "...KKKKKSSSSK.KSSSKKKKK...",
        "........KKKK...KKKK.......",
    ],
};

/// Seen from the side, facing right, standing.
pub const DOG_SIDE: Art = Art {
    w: 28,
    h: 30,
    rows: &[
        "............................",
        "............................",
        "............................",
        "............................",
        "............................",
        "............................",
        "............KK..............",
        "...........KPQK.............",
        "..........KPPQK.............",
        ".........KPPPQKKKKK.........",
        "........KWPPQWWWWWWKK.......",
        ".......KWWWWWWWWWWWWWK......",
        "......KWWWWWWWWWWWWWWWK.....",
        "......KWWWWWWWWWWEEWWWK.....",
        "......KWWWWWWWWWWEEWWWWKKK..",
        "......KWWWWWWWWWWWWWWWWWNNK.",
        "......KSWWWWWWWWWBBWWWWWNNK.",
        "......KSWWWWWWWWWWWWWWWKKK..",
        ".......KSWWWWWWWWWWWWWWK....",
        "........KSSWWWWWWWWWSSKK....",
        "...KK....KKHHHHHHHHKK.......",
        "..KWSK..KWWWWWWWWWWWK.......",
        "..KWSK.KWWWWWWWWWWWWSK......",
        "...KWSKWWWWWWWWWWWWWSK......",
        "....KWWWWWWWWWWWWWWWSK......",
        "....KSWWWWWWWWWWWWWSSK......",
        "....KSSKWSKKKKKKWSKSK.......",
        "....KWSKWSK....KWSKWSK......",
        "....KSSKSSK....KSSKSSK......",
        "....KKKKKKK....KKKKKKK......",
    ],
};

/// From the side, mid-step.
pub const DOG_SIDE_STEP: Art = Art {
    w: 28,
    h: 30,
    rows: &[
        "............................",
        "............................",
        "............................",
        "............................",
        "............................",
        "............................",
        "............KK..............",
        "...........KPQK.............",
        "..........KPPQK.............",
        ".........KPPPQKKKKK.........",
        "........KWPPQWWWWWWKK.......",
        ".......KWWWWWWWWWWWWWK......",
        "......KWWWWWWWWWWWWWWWK.....",
        "......KWWWWWWWWWWEEWWWK.....",
        "......KWWWWWWWWWWEEWWWWKKK..",
        "......KWWWWWWWWWWWWWWWWWNNK.",
        "......KSWWWWWWWWWBBWWWWWNNK.",
        "......KSWWWWWWWWWWWWWWWKKK..",
        ".......KSWWWWWWWWWWWWWWK....",
        "........KSSWWWWWWWWWSSKK....",
        "...KK....KKHHHHHHHHKK.......",
        "..KWSK..KWWWWWWWWWWWK.......",
        "..KWSK.KWWWWWWWWWWWWSK......",
        "...KWSKWWWWWWWWWWWWWSK......",
        "....KWWWWWWWWWWWWWWWSK......",
        "....KSWWWWWWWWWWWWWSSK......",
        "....KSSKSSKKKKKKSSKSK.......",
        "...KWSK.KWSK..KWSK.KWSK.....",
        "..KSSK...KSSKKSSK...KSSK....",
        "..KKKK...KKKKKKKK...KKKK....",
    ],
};

/// Seen from behind, standing.
pub const DOG_BACK: Art = Art {
    w: 26,
    h: 30,
    rows: &[
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "...KK................KK...",
        "..KSSK..............KSSK..",
        "..KSSSK............KSSSK..",
        "..KSSSSK..........KSSSSK..",
        "..KSSSSSKKKKKKKKKKSSSSSK..",
        "..KSSSWWWWWWWWWWWWWWSSSK..",
        "..KWSWWWWWWWWWWWWWWWWSWK..",
        ".KWWWWWWWWWWWWWWWWWWWWWWK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KSWWWWWWWWWWWWWWWWWWWSSK.",
        "..KSWWWWWWWWWWWWWWWWWSSK..",
        "...KSSWWWWWWWWWWWWWWSSK...",
        "....KKHHHHHHHHHHHHHHKK....",
        ".....KWWWWWWWWWWWWWWK.....",
        "....KWWWWWWWKKWWWWWWSK....",
        "....KWWWWWWKWSKWWWWWSK....",
        "...KWWSWWWWKWSKWWWWSWSK...",
        "...KWWSWWWWWKKWWWWWSWSK...",
        "...KWWSKWWWWWWWWWWKSWSK...",
        "...KSSSKSSSK..KSSSKSSSK...",
        "...KKKKKKKKK..KKKKKKKKK...",
    ],
};

/// From behind, mid-step.
pub const DOG_BACK_STEP: Art = Art {
    w: 26,
    h: 30,
    rows: &[
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        "...KK................KK...",
        "..KSSK..............KSSK..",
        "..KSSSK............KSSSK..",
        "..KSSSSK..........KSSSSK..",
        "..KSSSSSKKKKKKKKKKSSSSSK..",
        "..KSSSWWWWWWWWWWWWWWSSSK..",
        "..KWSWWWWWWWWWWWWWWWWSWK..",
        ".KWWWWWWWWWWWWWWWWWWWWWWK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KWWWWWWWWWWWWWWWWWWWWWSK.",
        ".KSWWWWWWWWWWWWWWWWWWWSSK.",
        "..KSWWWWWWWWWWWWWWWWWSSK..",
        "...KSSWWWWWWWWWWWWWWSSK...",
        "....KKHHHHHHHHHHHHHHKK....",
        ".....KWWWWWWWWWWWWWWK.....",
        "....KWWWWWWWKKWWWWWWSK....",
        "....KWWWWWWKWSKWWWWWSK....",
        "...KWWSWWWWKWSKWWWWSWSK...",
        "...KWWSWWWWWWWWWWWWSWSK...",
        "...KSSSKWWWWWWWWWWKSSSK...",
        "...KKKKKSSSK..KSSSKKKKK...",
        "........KKKK..KKKK........",
    ],
};

/// Front eyes: open.
pub const EYES_OPEN: Piece = Piece {
    x: 8,
    y: 14,
    art: Art {
        w: 10,
        h: 3,
        rows: &["EW......EW", "EE......EE", "EE......EE"],
    },
};

/// Front eyes: wow.
pub const EYES_WOW: Piece = Piece {
    x: 7,
    y: 13,
    art: Art {
        w: 12,
        h: 4,
        rows: &[
            "KEEK....KEEK",
            "EWWE....EWWE",
            "EEWE....EEWE",
            "KEEK....KEEK",
        ],
    },
};

/// Front eyes: heart.
pub const EYES_HEART: Piece = Piece {
    x: 7,
    y: 14,
    art: Art {
        w: 12,
        h: 3,
        rows: &["H.H.....H.H.", "HHH.....HHH.", ".H.......H.."],
    },
};

/// Front eyes: heart-big.
pub const EYES_HEART_BIG: Piece = Piece {
    x: 6,
    y: 13,
    art: Art {
        w: 12,
        h: 4,
        rows: &[
            "HH.HH..HH.HH",
            "HHHHH..HHHHH",
            ".HHH....HHH.",
            "..H......H..",
        ],
    },
};

/// Front eyes: happy.
pub const EYES_HAPPY: Piece = Piece {
    x: 7,
    y: 14,
    art: Art {
        w: 12,
        h: 2,
        rows: &[".EE......EE.", "E..E....E..E"],
    },
};

/// Front eyes: chill.
pub const EYES_CHILL: Piece = Piece {
    x: 7,
    y: 15,
    art: Art {
        w: 12,
        h: 2,
        rows: &["EEE......EEE", ".EE......EE."],
    },
};

/// Front eyes: lash.
pub const EYES_LASH: Piece = Piece {
    x: 6,
    y: 12,
    art: Art {
        w: 14,
        h: 4,
        rows: &[
            "K.K........K.K",
            ".EEE......EEE.",
            ".EWE......EWE.",
            ".EEE......EEE.",
        ],
    },
};

/// Front eyes: lash-happy.
pub const EYES_LASH_HAPPY: Piece = Piece {
    x: 6,
    y: 13,
    art: Art {
        w: 12,
        h: 3,
        rows: &["K..........K", "..EE....EE..", ".E..E..E..E."],
    },
};

/// Front mouth piece: smile.
pub const MOUTH_SMILE: Piece = Piece {
    x: 11,
    y: 17,
    art: Art {
        w: 4,
        h: 4,
        rows: &["NNNN", ".NN.", "K..K", ".KK."],
    },
};

/// Front mouth piece: wow.
pub const MOUTH_WOW: Piece = Piece {
    x: 11,
    y: 17,
    art: Art {
        w: 4,
        h: 6,
        rows: &["NNNN", ".NN.", ".KK.", "KTTK", "KTTK", ".KK."],
    },
};

/// Front mouth piece: grin.
pub const MOUTH_GRIN: Piece = Piece {
    x: 9,
    y: 17,
    art: Art {
        w: 8,
        h: 4,
        rows: &["..NNNN..", "...NN...", "K......K", ".KKKKKK."],
    },
};

/// Front mouth piece: laugh.
pub const MOUTH_LAUGH: Piece = Piece {
    x: 9,
    y: 17,
    art: Art {
        w: 8,
        h: 5,
        rows: &["..NNNN..", "...NN...", ".KKKKKK.", ".KTTTTK.", "..KKKK.."],
    },
};

/// Front mouth piece: blush.
pub const MOUTH_BLUSH: Piece = Piece {
    x: 4,
    y: 16,
    art: Art {
        w: 18,
        h: 1,
        rows: &["BB..............BB"],
    },
};

/// Her bow, from the front.
pub const BOW: Piece = Piece {
    x: 9,
    y: 6,
    art: Art {
        w: 8,
        h: 5,
        rows: &["KK.KK.KK", "KHHKKHHK", "KHHHHHHK", "KHHKKHHK", "KK....KK"],
    },
};

/// Worn from the front: glasses.
pub const WEAR_GLASSES: Piece = Piece {
    x: 7,
    y: 13,
    art: Art {
        w: 12,
        h: 4,
        rows: &[
            "KKKK....KKKK",
            "K..KKKKKK..K",
            "K..K....K..K",
            "KKKK....KKKK",
        ],
    },
};

/// Worn from the front: sunglasses.
pub const WEAR_SUNGLASSES: Piece = Piece {
    x: 6,
    y: 13,
    art: Art {
        w: 14,
        h: 4,
        rows: &[
            "KKKKKKKKKKKKKK",
            "KXXXXKKKKXXXXK",
            "KXXZXK..KXXZXK",
            ".KXXK....KXXK.",
        ],
    },
};

/// Worn from the front: 3d-glasses.
pub const WEAR_3D_GLASSES: Piece = Piece {
    x: 6,
    y: 13,
    art: Art {
        w: 14,
        h: 4,
        rows: &[
            "WWWWWWWWWWWWWW",
            "WRRRRWWWWUUUUW",
            "WRRRRW..WUUUUW",
            "WWWWWW..WWWWWW",
        ],
    },
};

/// Worn from the front: black-hat.
pub const WEAR_BLACK_HAT: Piece = Piece {
    x: 3,
    y: 2,
    art: Art {
        w: 20,
        h: 9,
        rows: &[
            "......KKKKKKKK......",
            ".....KXXXXXXXXK.....",
            "....KXXXXXXXXXXK....",
            "....KXXXXXXXXXXK....",
            "...KXXXXXXXXXXXXK...",
            "...KKKKKKKKKKKKKK...",
            ".KKXXXXXXXXXXXXXXKK.",
            "KXXXXXXXXXXXXXXXXXXK",
            ".KKKKKKKKKKKKKKKKKK.",
        ],
    },
};

/// Worn from the front: straw-hat.
pub const WEAR_STRAW_HAT: Piece = Piece {
    x: 1,
    y: 3,
    art: Art {
        w: 24,
        h: 8,
        rows: &[
            "........KKKKKKKK........",
            ".......KYYYYYYYYK.......",
            "......KYYyYYyYYYYK......",
            "......KHHHHHHHHHHK......",
            "..KKKKYYYYYYYYYYYYKKKK..",
            ".KYYYYyYYYYyYYYYyYYYYYK.",
            "KYYyYYYYYyYYYYYYYYyYYYYK",
            ".KKKKKKKKKKKKKKKKKKKKKK.",
        ],
    },
};

/// Side eyes: happy.
pub const SIDE_EYES_HAPPY: Piece = Piece {
    x: 16,
    y: 13,
    art: Art {
        w: 3,
        h: 2,
        rows: &["WEE", "EWW"],
    },
};

/// Side eyes: heart.
pub const SIDE_EYES_HEART: Piece = Piece {
    x: 16,
    y: 12,
    art: Art {
        w: 3,
        h: 3,
        rows: &["H.H", "HHH", "WHW"],
    },
};

/// Side eyes: closed.
pub const SIDE_EYES_CLOSED: Piece = Piece {
    x: 16,
    y: 13,
    art: Art {
        w: 3,
        h: 2,
        rows: &["WWW", "EEE"],
    },
};

/// Worn from the side: bow.
pub const SIDE_BOW: Piece = Piece {
    x: 11,
    y: 6,
    art: Art {
        w: 5,
        h: 4,
        rows: &["KK.KK", "KHKHK", "KHHHK", "KK.KK"],
    },
};

/// Prop: thought.
pub const PROP_THOUGHT: Art = Art {
    w: 18,
    h: 16,
    rows: &[
        "......KKKKKK......",
        "....KKWWWWWWKK....",
        "..KKWWWWWWWWWWKK..",
        ".KWWWWHH.HHWWWWWK.",
        "KWWWWHHHHHHHWWWWWK",
        "KWWWWWHHHHHWWWWWWK",
        ".KWWWWWHHHWWWWWWK.",
        "..KKWWWWHWWWWWKK..",
        "....KKWWWWWWKK....",
        "......KKKKKK......",
        "....KK............",
        "...KWWK...........",
        "....KK............",
        "..KK..............",
        ".KWK..............",
        "..K...............",
    ],
};

/// Prop: heart.
pub const PROP_HEART: Art = Art {
    w: 11,
    h: 9,
    rows: &[
        ".KKK...KKK.",
        "KHHHK.KHHHK",
        "KHWHHKHHHHK",
        "KHHHHHHHHHK",
        ".KHHHHHHHK.",
        "..KHHHHHK..",
        "...KHHHK...",
        "....KHK....",
        ".....K.....",
    ],
};

/// Prop: laptop.
pub const PROP_LAPTOP: Art = Art {
    w: 18,
    h: 11,
    rows: &[
        "..KKKKKKKKKKKKKK..",
        "..KAAAAAAAAAAAAK..",
        "..KAIIIIIIIIIIAK..",
        "..KAIGGIIIIIIIAK..",
        "..KAIIIIGGGIIIAK..",
        "..KAIGGGIIIIIIAK..",
        "..KAIIIIIIIIIIAK..",
        "..KAAAAAAAAAAAAK..",
        "KKKKKKKKKKKKKKKKKK",
        "KAAAAAAAAAAAAAAAAK",
        ".KKKKKKKKKKKKKKKK.",
    ],
};

/// Prop: sun.
pub const PROP_SUN: Art = Art {
    w: 17,
    h: 15,
    rows: &[
        "........Y........",
        "..Y.....Y.....Y..",
        "...Y..KKKKK..Y...",
        "....KKYYYYYKK....",
        "...KYYYYYYYYYK...",
        "..KYYYYYYYYYYYK..",
        "..KYYYYYYYYYYYK..",
        "YYKYYYYYYYYYYYKYY",
        "..KYYYYYYYYYYYK..",
        "..KYYyYYYYYyYYK..",
        "...KYYYYYYYYYK...",
        "....KKYYYYYKK....",
        "...Y..KKKKK..Y...",
        "..Y.....Y.....Y..",
        "........Y........",
    ],
};

/// Prop: popcorn.
pub const PROP_POPCORN: Art = Art {
    w: 13,
    h: 10,
    rows: &[
        "..CC.CCC.CC..",
        ".CCCCCCCCCCC.",
        "CCYCCCCCYCCCC",
        "KKKKKKKKKKKKK",
        "KRWRWRWRWRWRK",
        ".KRWRWRWRWRK.",
        ".KRWRWRWRWRK.",
        ".KRWRWRWRWRK.",
        "..KRWRWRWRK..",
        "..KKKKKKKKK..",
    ],
};

/// Prop: gelato.
pub const PROP_GELATO: Art = Art {
    w: 12,
    h: 12,
    rows: &[
        "........KK..",
        "...KKKKKnK..",
        "..KPPPPKnK..",
        ".KPWPPPPKK..",
        ".KPPPPMMMK..",
        "KPPPPMMWMMK.",
        "KKKKKKKKKKKK",
        "KWWHWWWWHWWK",
        ".KWWWWWWWWK.",
        ".KWWWHWWWWK.",
        "..KWWWWWWK..",
        "..KKKKKKKK..",
    ],
};

/// A white star, at its brightest.
pub const PROP_STAR: Art = Art {
    w: 7,
    h: 7,
    rows: &[
        "...W...", "...W...", "..WWW..", "WWWWWWW", "..WWW..", "...W...", "...W...",
    ],
};

/// A white star, small, as it twinkles on or off.
pub const PROP_STAR_SMALL: Art = Art {
    w: 5,
    h: 5,
    rows: &["..W..", ".WWW.", "WWWWW", ".WWW.", "..W.."],
};

/// Prop: sparkle.
pub const PROP_SPARKLE: Art = Art {
    w: 7,
    h: 7,
    rows: &[
        "...Y...", "...Y...", "..YYY..", "YYYWYYY", "..YYY..", "...Y...", "...Y...",
    ],
};

/// Prop: boba.
pub const PROP_BOBA: Art = Art {
    w: 10,
    h: 12,
    rows: &[
        "......KK..",
        ".....KHK..",
        ".KKKKKHKK.",
        "KWWWWWHWWK",
        "KKKKKKKKKK",
        ".KPPPPPPK.",
        ".KPWPPPPK.",
        ".KPPPPPPK.",
        ".KPPPPPPK.",
        ".KPEPEPPK.",
        ".KEPEPEPK.",
        "..KKKKKK..",
    ],
};

/// Prop: sushi.
pub const PROP_SUSHI: Art = Art {
    w: 14,
    h: 10,
    rows: &[
        "...KKKKKKKK...",
        "..KFFWFFWFFK..",
        ".KFWFFWFFWFFK.",
        ".KKKKKKKKKKKK.",
        ".KCCCCCCCCCCK.",
        ".KCCCCCCCCCCK.",
        "..KKKKKKKKKK..",
        "KKKKKKKKKKKKKK",
        "KnnnnnnnnnnnnK",
        ".KKKKKKKKKKKK.",
    ],
};

/// Prop: ramen.
pub const PROP_RAMEN: Art = Art {
    w: 17,
    h: 11,
    rows: &[
        "..........nn.....",
        ".........nn......",
        "........nn.......",
        "..KKKKKKKKKKKKK..",
        ".KYYYYCYYYYYYYYK.",
        "KYYCCYYYKOOKYYYYK",
        "KYYYYYYKOYYOKYYYK",
        "KKKKKKKKKKKKKKKKK",
        ".KRWRRRWRRRWRRRK.",
        "..KRRRRRRRRRRRK..",
        "...KKKKKKKKKKK...",
    ],
};

/// Prop: phone.
pub const PROP_PHONE: Art = Art {
    w: 7,
    h: 9,
    rows: &[
        "KKKKKKK", "KIIIIIK", "KIHIHIK", "KIHHHIK", "KIIHIIK", "KIIIIIK", "KIIIIIK", "KKKWKKK",
        ".KKKKK.",
    ],
};

/// Prop: matcha.
pub const PROP_MATCHA: Art = Art {
    w: 10,
    h: 12,
    rows: &[
        "......KK..",
        ".....KGK..",
        ".KKKKKGKK.",
        "KWWWWWGWWK",
        "KKKKKKKKKK",
        ".KWWWWWWK.",
        ".KMMMMMMK.",
        ".KMWMMMMK.",
        ".KMMMMMMK.",
        ".KGMGMMGK.",
        ".KMMMMMMK.",
        "..KKKKKK..",
    ],
};

/// Prop: note.
pub const PROP_NOTE: Art = Art {
    w: 7,
    h: 7,
    rows: &[
        "....KKK", "....KXK", "....K.K", "....K..", "..KKK..", ".KXXK..", ".KKK...",
    ],
};

/// Prop: leaf.
pub const PROP_LEAF: Art = Art {
    w: 6,
    h: 6,
    rows: &["....KK", "..KKOK", ".KOOOK", "KOOyK.", "KOyK..", "KK...."],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::color;

    fn all_art() -> impl Iterator<Item = (&'static str, Art)> {
        [
            ("DOG_FRONT", DOG_FRONT),
            ("DOG_FRONT_STEP", DOG_FRONT_STEP),
            ("DOG_SIDE", DOG_SIDE),
            ("DOG_SIDE_STEP", DOG_SIDE_STEP),
            ("DOG_BACK", DOG_BACK),
            ("DOG_BACK_STEP", DOG_BACK_STEP),
            ("PROP_THOUGHT", PROP_THOUGHT),
            ("PROP_HEART", PROP_HEART),
            ("PROP_LAPTOP", PROP_LAPTOP),
            ("PROP_SUN", PROP_SUN),
            ("PROP_POPCORN", PROP_POPCORN),
            ("PROP_GELATO", PROP_GELATO),
            ("PROP_BOBA", PROP_BOBA),
            ("PROP_SUSHI", PROP_SUSHI),
            ("PROP_RAMEN", PROP_RAMEN),
            ("PROP_PHONE", PROP_PHONE),
            ("PROP_MATCHA", PROP_MATCHA),
            ("PROP_NOTE", PROP_NOTE),
            ("PROP_LEAF", PROP_LEAF),
            ("PROP_SPARKLE", PROP_SPARKLE),
            ("PROP_STAR", PROP_STAR),
            ("PROP_STAR_SMALL", PROP_STAR_SMALL),
        ]
        .into_iter()
        .chain(
            [
                ("EYES_OPEN", EYES_OPEN),
                ("EYES_WOW", EYES_WOW),
                ("EYES_LASH", EYES_LASH),
                ("WEAR_STRAW_HAT", WEAR_STRAW_HAT),
                ("WEAR_BLACK_HAT", WEAR_BLACK_HAT),
            ]
            .into_iter()
            .map(|(n, p)| (n, p.art)),
        )
    }

    #[test]
    fn every_art_is_a_rectangle_of_known_colors() {
        for (name, a) in all_art() {
            assert_eq!(a.rows.len(), usize::from(a.h), "{name}");
            for row in a.rows {
                assert_eq!(row.len(), usize::from(a.w), "{name}: {row}");
                for c in row.bytes() {
                    assert!(c == b'.' || color(c).is_some(), "{name}: {}", char::from(c));
                }
            }
        }
    }

    #[test]
    fn faces_and_clothes_fit_on_the_dog_canvas() {
        let pieces = [
            EYES_OPEN,
            EYES_WOW,
            EYES_HEART,
            EYES_HEART_BIG,
            EYES_HAPPY,
            EYES_CHILL,
            EYES_LASH,
            EYES_LASH_HAPPY,
            MOUTH_SMILE,
            MOUTH_WOW,
            MOUTH_GRIN,
            MOUTH_LAUGH,
            MOUTH_BLUSH,
            BOW,
            WEAR_GLASSES,
            WEAR_SUNGLASSES,
            WEAR_3D_GLASSES,
            WEAR_BLACK_HAT,
            WEAR_STRAW_HAT,
        ];
        for p in pieces {
            assert!(p.x >= 0 && p.y >= 0);
            assert!(i32::from(p.x) + i32::from(p.art.w) <= i32::from(DOG_FRONT.w));
            assert!(i32::from(p.y) + i32::from(p.art.h) <= i32::from(DOG_FRONT.h));
        }
        for p in [SIDE_EYES_HAPPY, SIDE_EYES_HEART, SIDE_EYES_CLOSED, SIDE_BOW] {
            assert!(i32::from(p.x) + i32::from(p.art.w) <= i32::from(DOG_SIDE.w));
            assert!(i32::from(p.y) + i32::from(p.art.h) <= i32::from(DOG_SIDE.h));
        }
    }
}
