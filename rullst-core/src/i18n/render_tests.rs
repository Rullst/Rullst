use super::*;

const CATALOG: __Catalog = __Catalog {
    tags: &["en", "fr", "ja", "pt-BR"],
    plurals: &[
        __Plural::One,
        __Plural::ZeroOne,
        __Plural::Other,
        __Plural::ZeroOne,
    ],
    default: 0,
};

const GREETING: &[__Forms] = &[
    __Forms {
        one: &[],
        other: &[
            __Segment::Text("Hello, "),
            __Segment::Arg(0),
            __Segment::Text(" & "),
            __Segment::Arg(1),
        ],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Text("Bonjour "), __Segment::Arg(1)],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Arg(0), __Segment::Text("さん")],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Text("Olá, "), __Segment::Arg(0)],
    },
];

const FILES: &[__Forms] = &[
    __Forms {
        one: &[__Segment::Text("one file")],
        other: &[__Segment::Arg(0), __Segment::Text(" files")],
    },
    __Forms {
        one: &[__Segment::Arg(0), __Segment::Text(" fichier")],
        other: &[__Segment::Arg(0), __Segment::Text(" fichiers")],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Arg(0), __Segment::Text("件")],
    },
    __Forms {
        one: &[__Segment::Arg(0), __Segment::Text(" arquivo")],
        other: &[__Segment::Arg(0), __Segment::Text(" arquivos")],
    },
];

const HOME: &[__Forms] = &[
    __Forms {
        one: &[],
        other: &[__Segment::Text("Home")],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Text("Accueil")],
    },
    __Forms {
        one: &[],
        other: &[],
    },
    __Forms {
        one: &[],
        other: &[__Segment::Text("Início")],
    },
];

fn locale(tag: &'static str) -> Locale {
    Locale { tag }
}

fn files(tag: &'static str, count: u64) -> String {
    __text(&CATALOG, locale(tag), Some(count), FILES, &[&count]).into_owned()
}

#[test]
fn plural_rules_choose_one_for_their_counts() {
    assert!(!__Plural::One.is_one(0));
    assert!(__Plural::One.is_one(1));
    assert!(!__Plural::One.is_one(2));
    assert!(__Plural::ZeroOne.is_one(0));
    assert!(__Plural::ZeroOne.is_one(1));
    assert!(!__Plural::ZeroOne.is_one(2));
    assert!(!__Plural::Other.is_one(1));
    assert_eq!(files("en", 0), "0 files");
    assert_eq!(files("en", 1), "one file");
    assert_eq!(files("en", 2), "2 files");
    assert_eq!(files("fr", 0), "0 fichier");
    assert_eq!(files("fr", 1), "1 fichier");
    assert_eq!(files("fr", 2), "2 fichiers");
    assert_eq!(files("pt-BR", 0), "0 arquivo");
    assert_eq!(files("pt-BR", 1000), "1000 arquivos");
    assert_eq!(files("ja", 1), "1件");
}

#[test]
fn messages_interpolate_arguments_by_index() {
    let render = |tag| __text(&CATALOG, locale(tag), None, GREETING, &[&"Ana", &2]).into_owned();
    assert_eq!(render("en"), "Hello, Ana & 2");
    assert_eq!(render("fr"), "Bonjour 2");
    assert_eq!(render("ja"), "Anaさん");
    let missing = __text(&CATALOG, locale("en"), None, GREETING, &[&"Ana"]);
    assert_eq!(missing, "Hello, Ana & ");
}

#[test]
fn static_messages_are_borrowed() {
    let home = __text(&CATALOG, locale("pt-BR"), None, HOME, &[]);
    assert!(matches!(home, Cow::Borrowed("Início")));
    let empty = __text(&CATALOG, locale("ja"), None, HOME, &[]);
    assert!(matches!(empty, Cow::Borrowed("")));
    let formatted = __text(&CATALOG, locale("ja"), None, GREETING, &[&"Ana"]);
    assert!(matches!(formatted, Cow::Owned(_)));
}

#[test]
fn locales_outside_the_catalog_fall_back_by_language_then_default() {
    let home = |tag| __text(&CATALOG, locale(tag), None, HOME, &[]).into_owned();
    assert_eq!(home("pt-PT"), "Início");
    assert_eq!(home("fr-CA"), "Accueil");
    assert_eq!(home("de"), "Home");
    assert_eq!(files("pt-PT", 0), "0 arquivo");

    let short = __Catalog {
        tags: &["en", "es"],
        plurals: &[__Plural::One, __Plural::One],
        default: 0,
    };
    let only_one: &[__Forms] = &HOME[..1];
    assert_eq!(__text(&short, locale("es"), None, only_one, &[]), "Home");
    assert_eq!(__text(&short, locale("es"), None, &[], &[]), "");
    let plural_default = __Catalog {
        tags: &["en", "es"],
        plurals: &[__Plural::Other],
        default: 1,
    };
    assert_eq!(
        __text(&plural_default, locale("es"), Some(1), FILES, &[&1]),
        "1 fichiers"
    );
}

#[test]
fn html_messages_escape_arguments_only() {
    let forms: &[__Forms] = &[__Forms {
        one: &[],
        other: &[
            __Segment::Text("<a href=\"/terms\">"),
            __Segment::Arg(0),
            __Segment::Arg(3),
            __Segment::Text("</a>"),
        ],
    }];
    let catalog = __Catalog {
        tags: &["en"],
        plurals: &[__Plural::One],
        default: 0,
    };
    let markup = __html(&catalog, locale("en"), None, forms, &[&"<b>&'\"</b>"]);
    assert_eq!(
        markup.as_str(),
        "<a href=\"/terms\">&lt;b&gt;&amp;&#x27;&quot;&lt;/b&gt;</a>"
    );
}

#[test]
fn locale_arguments_may_be_borrowed() {
    let pt = locale("pt-BR");
    assert_eq!(__locale(&pt), pt);
    assert_eq!(__locale(&&pt), pt);
}
