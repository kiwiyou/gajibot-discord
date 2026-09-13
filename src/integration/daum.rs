use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Request, Version};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

#[derive(Clone)]
pub struct DaumClient {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
}

impl Default for DaumClient {
    fn default() -> Self {
        let tls_config = rustls::ClientConfig::builder()
            .try_with_platform_verifier()
            .unwrap()
            .with_no_client_auth();
        let https_config = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_http2()
            .build();
        let client =
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https_config);
        Self { client }
    }
}

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

const QUERY_ENCODE_SET: AsciiSet = CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

impl DaumClient {
    pub async fn search_general(&self, query: &str) -> Option<GeneralSearch> {
        let uri = format!(
            "https://dic.daum.net/search.do?dic=all&q={}",
            utf8_percent_encode(query, &QUERY_ENCODE_SET),
        );
        let request = Request::builder()
            .version(Version::HTTP_2)
            .uri(&uri)
            .method("GET")
            .body(Full::default())
            .unwrap();
        let body = self
            .client
            .request(request)
            .await
            .unwrap()
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        let html = String::from_utf8(body).unwrap();
        let typo = html
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
        let (_, dict_start) = html.split_once(r#"tit_word">"#)?;
        let (_, lemma_start) = dict_start.split_once(r#"표제어 클릭">"#).unwrap();
        let (dirty_lemma, after_lemma) = lemma_start.split_once("</a>")?;
        let hit = if dict_start.starts_with("영어사전") || dict_start.starts_with("영영사전")
        {
            // sup example: A²
            let lemma = unhtml_sup(&remove_simple_tag(
                dirty_lemma,
                r#"<span class="txt_emph1">"#,
                "</span>",
            ));
            let (list_search, after_list) = after_lemma.split_once("</ul>").unwrap();
            let definitions = parse_list_search(list_search);
            let phonetics = after_list
                .split_once(r#"txt_pronounce">"#)
                .map(|(_, phonetics)| {
                    let (american, after_list) = phonetics.split_once("</span>").unwrap();
                    let (_, phonetics) = after_list.split_once(r#"txt_pronounce">"#).unwrap();
                    let (british, _) = phonetics.split_once("</span>").unwrap();
                    // Λ example: stunning [stΛniŋ]
                    (
                        unhtml_phonetics(american).replace('Λ', "ʌ"),
                        unhtml_phonetics(british).replace('Λ', "ʌ"),
                    )
                });
            GeneralSearchHit::En {
                lemma,
                definitions,
                phonetics,
            }
        } else if dict_start.starts_with("한국어사전") {
            let mut lemma = unhtml_sup(&remove_simple_tag(
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
            let phonetics =
                list_search
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
        definitions.push(unhtml_sup(&definition));
    }
    definitions
}

fn unhtml_sup(mut text: &str) -> String {
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

fn unhtml_phonetics(phonetics: &str) -> String {
    // example: author [<daum:pron>ɔ́</daum:pron>ːθ<daum:pron>ə</daum:pron><daum:italic>r</daum:italic>]
    let buf = remove_simple_tag(phonetics, "<daum:pron>", "</daum:pron>");
    buf.replace("<daum:italic>r</daum:italic>", "𝘳")
}
