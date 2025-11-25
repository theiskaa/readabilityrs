//! Constants, regular expressions, and static data used throughout the library.

use once_cell::sync::Lazy;
use regex::Regex;

// Bitflags for parsing strategies
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ParseFlags: u32 {
        const STRIP_UNLIKELYS = 0x1;
        const WEIGHT_CLASSES = 0x2;
        const CLEAN_CONDITIONALLY = 0x4;
    }
}

// Element tags to score by default
// Note: DIV is included because many modern websites use DIVs for paragraphs
pub static DEFAULT_TAGS_TO_SCORE: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "SECTION", "H2", "H3", "H4", "H5", "H6", "P", "TD", "PRE", "DIV",
    ]
});

// Regular expressions (compiled once)
pub static REGEXPS: Lazy<RegexPatterns> = Lazy::new(RegexPatterns::new);

pub struct RegexPatterns {
    pub unlikely_candidates: Regex,
    pub ok_maybe_its_a_candidate: Regex,
    pub positive: Regex,
    pub negative: Regex,
    pub byline: Regex,
    pub normalize: Regex,
    pub videos: Regex,
    pub hash_url: Regex,
    pub commas: Regex,
    pub json_ld_article_types: Regex,
    pub ad_words: Regex,
    pub loading_words: Regex,
}

impl RegexPatterns {
    fn new() -> Self {
        Self {
            unlikely_candidates: Regex::new(
                r"(?i)-ad-|ai2html|banner|breadcrumbs|combx|comment|community|cover-wrap|disqus|extra|footer|gdpr|header|legends|menu|related|remark|replies|rss|shoutbox|sidebar|skyscraper|social|sponsor|supplemental|ad-break|agegate|pagination|pager|popup|yom-remote"
            ).unwrap(),
            ok_maybe_its_a_candidate: Regex::new(
                r"(?i)and|article|body|column|content|main|mathjax|shadow"
            ).unwrap(),
            positive: Regex::new(
                r"(?i)article|body|content|entry|hentry|h-entry|main|page|pagination|post|text|blog|story"
            ).unwrap(),
            negative: Regex::new(
                r"(?i)-ad-|hidden|^hid$| hid$| hid |^hid |banner|combx|comment|com-|contact|footer|gdpr|masthead|media|meta|outbrain|promo|related|scroll|share|shoutbox|sidebar|skyscraper|sponsor|shopping|tags|widget"
            ).unwrap(),
            byline: Regex::new(
                r"(?i)byline|author|dateline|writtenby|p-author"
            ).unwrap(),
            normalize: Regex::new(
                r"\s{2,}"
            ).unwrap(),
            videos: Regex::new(
                r"(?i)//(www\.)?((dailymotion|youtube|youtube-nocookie|player\.vimeo|v\.qq|bilibili|live.bilibili)\.com|(archive|upload\.wikimedia)\.org|player\.twitch\.tv)"
            ).unwrap(),
            hash_url: Regex::new(
                r"^#.+"
            ).unwrap(),
            commas: Regex::new(
                "[\u{002C}\u{060C}\u{FE50}\u{FE10}\u{FE11}\u{2E41}\u{2E34}\u{2E32}\u{FF0C}]"
            ).unwrap(),
            json_ld_article_types: Regex::new(
                r"^Article|AdvertiserContentArticle|NewsArticle|AnalysisNewsArticle|AskPublicNewsArticle|BackgroundNewsArticle|OpinionNewsArticle|ReportageNewsArticle|ReviewNewsArticle|Report|SatiricalArticle|ScholarlyArticle|MedicalScholarlyArticle|SocialMediaPosting|BlogPosting|LiveBlogPosting|DiscussionForumPosting|TechArticle|APIReference$"
            ).unwrap(),
            ad_words: Regex::new(
                r"(?iu)^(ad(vertising|vertisement)?|pub(licité)?|werb(ung)?|广告|Реклама|Anuncio)$"
            ).unwrap(),
            loading_words: Regex::new(
                r"(?iu)^((loading|正在加载|Загрузка|chargement|cargando)(…|\.\.\.)?)$"
            ).unwrap(),
        }
    }
}

// Elements that can be converted from DIV to P
pub const DIV_TO_P_ELEMS: &[&str] = &[
    "BLOCKQUOTE",
    "DL",
    "DIV",
    "IMG",
    "OL",
    "P",
    "PRE",
    "TABLE",
    "UL",
];


// Phrasing (inline) elements
pub const PHRASING_ELEMS: &[&str] = &[
    "ABBR", "AUDIO", "B", "BDO", "BR", "BUTTON", "CITE", "CODE", "DATA", "DATALIST", "DFN",
    "EM", "EMBED", "I", "IMG", "INPUT", "KBD", "LABEL", "MARK", "MATH", "METER", "NOSCRIPT",
    "OBJECT", "OUTPUT", "PROGRESS", "Q", "RUBY", "SAMP", "SCRIPT", "SELECT", "SMALL", "SPAN",
    "STRONG", "SUB", "SUP", "TEXTAREA", "TIME", "VAR", "WBR",
];
