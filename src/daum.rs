pub struct GeneralSearch {
    pub typo: Vec<String>,
    pub hit: GeneralSearchHit,
}

pub enum GeneralSearchHit {
    En {
        lemma: String,
        definitions: Vec<String>,
        /// (AmE, BrE)
        phonetics: Option<(String, String)>,
    },
    Ko {
        lemma: String,
        definitions: Vec<String>,
        foreign: Option<String>,
        phonetics: Option<String>,
    },
}

pub fn search_general(agent: &ureq::Agent, query: &str) -> Option<GeneralSearch> {
    let body = agent
        .get("https://dic.daum.net/search.do")
        .query_pairs([("dic", "all"), ("q", query)])
        .call()
        .unwrap()
        .into_body()
        .read_to_string()
        .unwrap();
    let typo = body
        .split_once("tit_speller")
        .map_or_default(|(_, speller_begin)| {
            let (tit_speller, _) = speller_begin.split_once("</div>").unwrap();
            tit_speller
                .split(r#"검색하기">"#)
                .skip(1)
                .map(|typo_begin| {
                    let (typo, _) = typo_begin.split_once("</a>").unwrap();
                    typo.into()
                })
                .collect()
        });
    let (_, dict_start) = body.split_once(r#"tit_word">"#)?;
    let (_, lemma_start) = dict_start.split_once(r#"표제어 클릭">"#).unwrap();
    let (dirty_lemma, after_lemma) = lemma_start.split_once("</a>")?;
    let hit = if dict_start.starts_with("영어사전") || dict_start.starts_with("영영사전") {
        let lemma = remove_simple_tag(dirty_lemma, r#"<span class="txt_emph1">"#, "</span>");
        let (list_search, after_list) = after_lemma.split_once("</ul>").unwrap();
        let definitions = parse_list_search(list_search);
        let phonetics = after_list
            .split_once(r#"txt_pronounce">"#)
            .map(|(_, phonetics)| {
                let (american, after_list) = phonetics.split_once("</span>").unwrap();
                let (_, phonetics) = after_list.split_once(r#"txt_pronounce">"#).unwrap();
                let (british, _) = phonetics.split_once("</span>").unwrap();
                (
                    remove_simple_tag(american, "<daum:pron>", "</daum:pron>"),
                    remove_simple_tag(british, "<daum:pron>", "</daum:pron>"),
                )
            });
        GeneralSearchHit::En {
            lemma,
            definitions,
            phonetics,
        }
    } else if dict_start.starts_with("한국어사전") {
        let mut lemma = convert_sup(&remove_simple_tag(
            dirty_lemma,
            r#"<span class="txt_emph1">"#,
            "</span>",
        ));
        if let Some(sup) = lemma.find("<sup") {
            lemma.truncate(sup);
        }
        let (list_search, _) = after_lemma.split_once("</ul>").unwrap();
        let definitions = parse_list_search(list_search);
        let foreign = list_search
            .split_once(r#"sub_txt">"#)
            .map(|(_, foreign_start)| {
                let (foreign, _) = foreign_start.split_once("</span>").unwrap();
                foreign.trim().into()
            });
        let phonetics = list_search
            .split_once(r#"txt_pronounce">"#)
            .map(|(_, phonetics_start)| {
                let (phonetics, _) = phonetics_start.split_once("</span>").unwrap();
                phonetics.into()
            });
        GeneralSearchHit::Ko {
            lemma,
            definitions,
            foreign,
            phonetics,
        }
    } else {
        return None;
    };
    Some(GeneralSearch { typo, hit })
}

fn remove_simple_tag(mut dirty: &str, tag_begin: &str, tag_end: &str) -> String {
    let mut buf = String::new();
    while let Some((before_tag, rest)) = dirty.split_once(tag_begin) {
        buf.push_str(before_tag);
        let (inside_tag, rest) = rest.split_once(tag_end).unwrap();
        buf.push_str(inside_tag);
        dirty = rest;
    }
    buf.push_str(dirty);
    buf
}

fn parse_list_search(mut list_search: &str) -> Vec<String> {
    let mut definitions = vec![];
    while let Some((_, definition_start)) = list_search.split_once(r#"txt_search">"#) {
        let (dirty_definition, rest) = definition_start.split_once("</span>").unwrap();
        list_search = rest;
        let definition = {
            let mut dirty_definition = dirty_definition;
            let mut buf = String::new();
            while let Some((before_link, rest)) = dirty_definition.split_once("<daum:") {
                buf.push_str(before_link);
                let (_, rest) = rest.split_once('>').unwrap();
                let (inside_link, rest) = rest.split_once("</daum:word>").unwrap();
                buf.push_str(inside_link);
                dirty_definition = rest;
            }
            buf.push_str(dirty_definition);
            buf
        };
        definitions.push(convert_sup(&definition));
    }
    definitions
}

fn convert_sup(mut text: &str) -> String {
    let mut buf = String::new();
    while let Some((before_sup, rest)) = text.split_once("<sup>") {
        buf.push_str(before_sup);
        let (inside_sup, rest) = rest.split_once("</sup>").unwrap();
        for byte in inside_sup.bytes() {
            buf.push(match byte {
                b'0' => '⁰',
                b'1' => '¹',
                b'2' => '²',
                b'3' => '³',
                b'4' => '⁴',
                b'5' => '⁵',
                b'6' => '⁶',
                b'7' => '⁷',
                b'8' => '⁸',
                b'9' => '⁹',
                _ => continue,
            });
        }
        text = rest;
    }
    buf.push_str(text);
    buf
}
