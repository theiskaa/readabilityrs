//! Content cleaning and post-processing functions.

use crate::constants::{DIV_TO_P_ELEMS, REGEXPS};
use crate::error::Result;
use kuchikikiki::{traits::*, NodeData, NodeRef};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use scraper::{ElementRef, Html, Selector};

/// Clean and post-process extracted article content (light version)
///
/// This function:
/// - Fixes relative URLs to absolute
/// - Removes nav-like sections
pub fn clean_article_content_light(html: &str, base_url: Option<&str>) -> Result<String> {
    let mut result = html.to_string();

    if let Some(base) = base_url {
        result = fix_relative_urls_in_html(&result, base);
    }

    result = remove_nav_like_sections(&result);

    Ok(result)
}

/// Clean and post-process extracted article content (full version)
///
/// This function:
/// - Removes unwanted elements (scripts, styles, forms, etc.)
/// - Fixes relative URLs to absolute
/// - Cleans up empty elements
/// - Normalizes whitespace
pub fn clean_article_content(html: &str, base_url: Option<&str>) -> Result<String> {
    let mut result = clean_article_content_light(html, base_url)?;
    result = remove_conditionally(&result);
    Ok(result)
}

/// Fix relative URLs in HTML string using regex
fn fix_relative_urls_in_html(html: &str, _base_url: &str) -> String {
    // For now, just return as-is
    // TODO: Implement proper URL fixing without re-parsing the entire tree
    html.to_string()
}

/// Remove nav-like sections using lightweight regex patterns.
fn remove_nav_like_sections(html: &str) -> String {
    static NAV_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?is)<nav\b[^>]*?>.*?</nav>").unwrap());

    let mut result = NAV_REGEX.replace_all(html, "").to_string();

    let tags = ["div", "section", "ul", "ol"];
    // Note: "widget" is intentionally excluded from this regex-based removal because
    // page builders (Elementor, Divi, etc.) use "widget" in class names for ALL content
    // containers. Widgets with negative class weight are handled by should_remove_dom_node
    // which also considers content quality (link density, text length).
    let keywords = ["nav", "navbar", "menu", "breadcrumbs", "sidebar"];

    for tag in tags {
        for keyword in keywords {
            let class_pattern = format!(
                r#"(?is)<{tag}\b[^>]*?class="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#
            );
            let re = Regex::new(&class_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();

            let id_pattern = format!(
                r#"(?is)<{tag}\b[^>]*?id="[^"]*?{keyword}[^"]*?"[^>]*?>.*?</{tag}>"#
            );
            let re = Regex::new(&id_pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
    }

    result
}

fn remove_conditionally(html: &str) -> String {
    remove_conditionally_dom(html).unwrap_or_else(|| remove_conditionally_regex(html))
}

fn remove_conditionally_dom(html: &str) -> Option<String> {
    let document = kuchikikiki::parse_html().one(html);
    let body_node = document
        .select("body")
        .ok()
        .and_then(|mut iter| iter.next())
        .map(|node| node.as_node().clone());

    let (target_node, children_only) = if let Some(body) = body_node.clone() {
        (body, true)
    } else {
        (document.clone(), false)
    };
    mark_data_tables(&target_node);

    let cleanup_tags = ["form", "fieldset", "table", "ul", "ol", "div", "section"];
    for tag in cleanup_tags {
        clean_conditionally_tag(&target_node, tag);
    }

    Some(serialize_node(&target_node, children_only))
}

fn serialize_node(node: &NodeRef, children_only: bool) -> String {
    let mut buffer = Vec::new();

    if children_only {
        let children: Vec<_> = node.children().collect();
        for child in children {
            if child.serialize(&mut buffer).is_err() {
                return node.text_contents();
            }
        }
    } else if node.serialize(&mut buffer).is_err() {
        return node.text_contents();
    }

    String::from_utf8(buffer).unwrap_or_else(|_| node.text_contents())
}

fn clean_conditionally_tag(root: &NodeRef, tag: &str) {
    if let Ok(matches) = root.select(tag) {
        let nodes: Vec<_> = matches
            .map(|css_match| css_match.as_node().clone())
            .collect();
        for node in nodes {
            if should_remove_dom_node(&node, tag) {
                node.detach();
            }
        }
    }
}

fn should_remove_dom_node(node: &NodeRef, tag: &str) -> bool {
    let trimmed = node.text_contents().trim().to_string();
    if trimmed.len() > 600 {
        return false;
    }

    let mut is_list = tag.eq_ignore_ascii_case("ul") || tag.eq_ignore_ascii_case("ol");
    if !is_list {
        let node_text_len = trimmed.len().max(1);
        let list_text_len = node
            .select("ul, ol")
            .ok()
            .map(|iter| {
                iter.map(|n| dom_inner_text(&n.as_node().clone()).len())
                    .sum::<usize>()
            })
            .unwrap_or(0);
        is_list = (list_text_len as f64 / node_text_len as f64) > 0.9;
    }

    if tag.eq_ignore_ascii_case("table") && is_data_table(node) {
        return false;
    }

    if has_ancestor(node, |ancestor| {
        is_table(ancestor) && is_data_table(ancestor)
    }) {
        return false;
    }

    if has_ancestor(node, |ancestor| node_has_tag(ancestor, "code")) {
        return false;
    }

    if node_contains_data_table(node) {
        return false;
    }

    let content_length = trimmed.len();
    let link_density = dom_link_density(node, content_length);

    let weight = get_dom_class_weight(node);
    // Don't remove based solely on negative class weight. Also require high link density
    // or very short content. This prevents removing legitimate content in page builders
    // (like Elementor, Divi, etc.) that use generic class names like "widget" for
    // content containers, not just sidebar widgets.
    if weight < 0 && (link_density > 0.25 || content_length < 100) {
        return true;
    }

    if trimmed.matches(',').count() >= 10 {
        return false;
    }

    let p = count_descendants(node, "p");
    let img = count_descendants(node, "img");
    let li = count_descendants(node, "li").saturating_sub(100);
    let input = count_descendants(node, "input");
    let heading_density = get_text_density(node, &["h1", "h2", "h3", "h4", "h5", "h6"]);

    let mut embed_count = 0;
    if let Ok(embeds) = node.select("object, embed, iframe") {
        for embed in embeds {
            let embed_node = embed.as_node();
            if node_has_allowed_video(embed_node) {
                return false;
            }
            embed_count += 1;
        }
    }

    if REGEXPS.ad_words.is_match(trimmed.trim()) || REGEXPS.loading_words.is_match(trimmed.trim()) {
        return true;
    }
    let text_density = get_text_density(node, &build_textish_tags());
    let is_figure_child = has_ancestor(node, |ancestor| node_has_tag(ancestor, "figure"));

    let comma_count = trimmed.matches(',').count();

    if comma_count >= 10 {
        return false;
    }

    let mut should_remove = false;
    if !is_figure_child && img > 1 && p > 0 && (p as f64 / img as f64) < 0.5 {
        should_remove = true;
    }
    if !is_list && li > p {
        should_remove = true;
    }
    if input > p.saturating_div(3) {
        should_remove = true;
    }
    if !is_list
        && !is_figure_child
        && heading_density < 0.9
        && content_length < 25
        && link_density > 0.0
    {
        should_remove = true;
    }
    if !is_list && weight < 25 && link_density > 0.2 {
        should_remove = true;
    }
    if weight >= 25 && link_density > 0.5 {
        should_remove = true;
    }
    if (embed_count == 1 && content_length < 75) || embed_count > 1 {
        should_remove = true;
    }
    if img == 0 && text_density == 0.0 {
        should_remove = true;
    }

    if is_list && should_remove {
        let simple_children = node.children().all(|child| {
            if child.as_element().is_none() {
                return true;
            }
            child
                .children()
                .filter(|n| n.as_element().is_some())
                .count()
                <= 1
        });
        if simple_children {
            let li_count = count_descendants(node, "li");
            if li_count > 0 && img == li_count {
                should_remove = false;
            }
        }
    }

    should_remove
}

fn dom_link_density(node: &NodeRef, text_len: usize) -> f64 {
    if text_len == 0 {
        return 1.0;
    }

    if let Ok(links) = node.select("a") {
        let mut link_length = 0;
        for link in links {
            link_length += link.as_node().text_contents().len();
        }
        link_length as f64 / text_len as f64
    } else {
        0.0
    }
}

fn dom_inner_text(node: &NodeRef) -> String {
    node.text_contents()
}

fn mark_data_tables(root: &NodeRef) {
    if let Ok(tables) = root.select("table") {
        for table_sel in tables {
            let table = table_sel.as_node();
            let is_data = detect_data_table(table);
            set_data_table_flag(table, is_data);
        }
    }
}

fn detect_data_table(table: &NodeRef) -> bool {
    if let Some(element) = table.as_element() {
        let attrs = element.attributes.borrow();
        if matches!(attrs.get("role"), Some(role) if role == "presentation") {
            return false;
        }
        if matches!(attrs.get("datatable"), Some(val) if val == "0") {
            return false;
        }
        if attrs.get("summary").is_some() {
            return true;
        }
    }

    if table
        .select("caption")
        .ok()
        .and_then(|mut c| c.next())
        .is_some()
    {
        return true;
    }

    let has_data_descendant = ["col", "colgroup", "tfoot", "thead", "th"]
        .iter()
        .any(|tag| table.select(tag).ok().and_then(|mut c| c.next()).is_some());
    if has_data_descendant {
        return true;
    }

    if table
        .select("table")
        .ok()
        .and_then(|mut c| c.next())
        .is_some()
    {
        return false;
    }

    let (rows, columns) = get_row_and_column_count(table);
    if rows == 0 || columns == 0 {
        return false;
    }
    if rows == 1 || columns == 1 {
        return false;
    }
    if rows >= 10 || columns > 4 {
        return true;
    }
    rows * columns > 10
}

fn get_row_and_column_count(table: &NodeRef) -> (usize, usize) {
    let mut rows = 0;
    let mut columns = 0;
    if let Ok(trs) = table.select("tr") {
        for tr in trs {
            rows += 1;
            let cols = tr
                .as_node()
                .children()
                .filter(|child| {
                    if let Some(elem) = child.as_element() {
                        let name = elem.name.local.as_ref().to_ascii_lowercase();
                        name == "td" || name == "th"
                    } else {
                        false
                    }
                })
                .count();
            columns = columns.max(cols);
        }
    }
    (rows, columns)
}

fn set_data_table_flag(node: &NodeRef, is_data: bool) {
    if let Some(element) = node.as_element() {
        let mut attrs = element.attributes.borrow_mut();
        let value = if is_data { "true" } else { "false" }.to_string();
        attrs.insert("data-readability-datatable", value);
    }
}

fn is_data_table(node: &NodeRef) -> bool {
    if let Some(element) = node.as_element() {
        let attrs = element.attributes.borrow();
        matches!(
            attrs.get("data-readability-datatable"),
            Some(value) if value == "true"
        )
    } else {
        false
    }
}

fn node_contains_data_table(node: &NodeRef) -> bool {
    if let Ok(tables) = node.select("table") {
        for table in tables {
            if is_data_table(&table.as_node().clone()) {
                return true;
            }
        }
    }
    false
}

fn has_ancestor<F>(node: &NodeRef, mut predicate: F) -> bool
where
    F: FnMut(&NodeRef) -> bool,
{
    let mut current = node.parent();
    while let Some(parent) = current {
        if let NodeData::Element(_) = parent.data() {
            if predicate(&parent) {
                return true;
            }
        }
        current = parent.parent();
    }
    false
}

fn node_has_tag(node: &NodeRef, tag: &str) -> bool {
    if let Some(element) = node.as_element() {
        element.name.local.as_ref().eq_ignore_ascii_case(tag)
    } else {
        false
    }
}

fn is_table(node: &NodeRef) -> bool {
    node_has_tag(node, "table")
}

fn count_descendants(node: &NodeRef, selector: &str) -> usize {
    node.select(selector).map(|iter| iter.count()).unwrap_or(0)
}

fn node_has_allowed_video(node: &NodeRef) -> bool {
    if let Some(element) = node.as_element() {
        let attrs = element.attributes.borrow();
        for (_, attribute) in attrs.map.iter() {
            if REGEXPS.videos.is_match(&attribute.value) {
                return true;
            }
        }
    }
    if node_has_tag(node, "object") && REGEXPS.videos.is_match(&node.text_contents()) {
        return true;
    }
    false
}

fn build_textish_tags() -> Vec<&'static str> {
    let mut tags = vec!["span", "li", "td"];
    for tag in DIV_TO_P_ELEMS.iter() {
        tags.push(tag);
    }
    tags
}

fn get_text_density(node: &NodeRef, tags: &[&str]) -> f64 {
    let total_text = dom_inner_text(node).len() as f64;
    if total_text == 0.0 {
        return 0.0;
    }

    let mut child_text = 0.0;
    for tag in tags {
        if let Ok(matches) = node.select(tag) {
            for child in matches {
                child_text += dom_inner_text(&child.as_node().clone()).len() as f64;
            }
        }
    }
    child_text / total_text
}

fn get_dom_class_weight(node: &NodeRef) -> i32 {
    let mut weight = 0;
    if let Some(element) = node.as_element() {
        let attrs = element.attributes.borrow();
        if let Some(class) = attrs.get("class") {
            if REGEXPS.negative.is_match(class) {
                weight -= 25;
            }
            if REGEXPS.positive.is_match(class) {
                weight += 25;
            }
        }
        if let Some(id) = attrs.get("id") {
            if REGEXPS.negative.is_match(id) {
                weight -= 25;
            }
            if REGEXPS.positive.is_match(id) {
                weight += 25;
            }
        }
    }
    weight
}

fn remove_conditionally_regex(html: &str) -> String {
    let mut result = html.to_string();
    let cleanup_tags = ["table", "ul", "ol", "div", "section"];

    for tag in cleanup_tags {
        result = remove_blocks_for_tag(&result, tag);
    }

    result
}

fn remove_blocks_for_tag(html: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}\b[^>]*?>.*?</{tag}>");
    let re = Regex::new(&pattern).unwrap();

    re.replace_all(html, |caps: &Captures| {
        let block = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        if should_remove_block(block, tag) {
            String::new()
        } else {
            block.to_string()
        }
    })
    .to_string()
}

static WRAPPER_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.__rrs_conditional_wrapper").unwrap());
static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("a").unwrap());
static P_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("p").unwrap());
static IMG_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("img").unwrap());
static LI_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("li").unwrap());
static INPUT_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("input").unwrap());
static IFRAME_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("iframe").unwrap());
static EMBED_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("embed").unwrap());
static OBJECT_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("object").unwrap());
static H1_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h1").unwrap());
static H2_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h2").unwrap());
static H3_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h3").unwrap());
static H4_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h4").unwrap());
static H5_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h5").unwrap());
static H6_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h6").unwrap());

fn should_remove_block(fragment: &str, tag: &str) -> bool {
    let stats = compute_fragment_stats(fragment);

    if stats.text_len > 600 {
        return false;
    }

    let is_list = tag.eq_ignore_ascii_case("ul")
        || tag.eq_ignore_ascii_case("ol")
        || (stats.counts.li > 0 && stats.counts.li as f64 / stats.counts.p.max(1) as f64 > 1.5);

    if !stats.class_id.is_empty() {
        let class_id = stats.class_id.as_str();
        // Note: "widget" is excluded here since page builders use it for content containers.
        // Widgets are handled by should_remove_dom_node with content quality checks.
        if (class_id.contains("nav")
            || class_id.contains("menu")
            || class_id.contains("sidebar")
            || class_id.contains("related")
            || class_id.contains("sponsored"))
            && (stats.link_density > 0.1
                || stats.text_len < 400
                || tag.eq_ignore_ascii_case("table"))
        {
            return true;
        }
    }

    if stats.text_len == 0 && stats.link_density >= 0.2 {
        return true;
    }

    if stats.link_density > 0.55 {
        return true;
    }

    if !is_list
        && stats.counts.img > 1
        && stats.counts.p > 0
        && (stats.counts.p as f64 / stats.counts.img as f64) < 0.5
    {
        return true;
    }

    if !is_list && stats.counts.li > stats.counts.p && stats.counts.li > 0 {
        return true;
    }

    if stats.counts.inputs > 0 && stats.counts.inputs * 3 > stats.counts.p.max(1) {
        return true;
    }

    if stats.counts.headings > stats.counts.p && stats.comma_count < 2 && stats.text_len < 150 {
        return true;
    }

    if stats.counts.embeds > 1 && stats.text_len < 220 {
        return true;
    }

    false
}

#[derive(Default)]
struct ChildCounts {
    p: usize,
    img: usize,
    li: usize,
    inputs: usize,
    embeds: usize,
    headings: usize,
}

struct FragmentStats {
    text_len: usize,
    link_density: f64,
    counts: ChildCounts,
    comma_count: usize,
    class_id: String,
}

fn compute_fragment_stats(fragment: &str) -> FragmentStats {
    let wrapped = format!(
        "<html><body><div class=\"__rrs_conditional_wrapper\">{fragment}</div></body></html>"
    );
    let document = Html::parse_document(&wrapped);

    let wrapper = document.select(&WRAPPER_SELECTOR).next();

    let text = wrapper
        .as_ref()
        .map(|node| node.text().collect::<String>())
        .unwrap_or_default();
    let text_len = text.trim().len();
    let comma_count = text.matches(',').count();

    let mut link_length = 0;
    if let Some(node) = wrapper.as_ref() {
        for link in node.select(&LINK_SELECTOR) {
            link_length += link.text().collect::<String>().len();
        }
    }

    let counts = ChildCounts {
        p: count_in_wrapper(&wrapper, &P_SELECTOR),
        img: count_in_wrapper(&wrapper, &IMG_SELECTOR),
        li: count_in_wrapper(&wrapper, &LI_SELECTOR),
        inputs: count_in_wrapper(&wrapper, &INPUT_SELECTOR),
        embeds: count_multi_in_wrapper(
            &wrapper,
            &[&IFRAME_SELECTOR, &EMBED_SELECTOR, &OBJECT_SELECTOR],
        ),
        headings: count_multi_in_wrapper(
            &wrapper,
            &[
                &H1_SELECTOR,
                &H2_SELECTOR,
                &H3_SELECTOR,
                &H4_SELECTOR,
                &H5_SELECTOR,
                &H6_SELECTOR,
            ],
        ),
    };

    FragmentStats {
        text_len,
        link_density: if text_len == 0 {
            1.0
        } else {
            link_length as f64 / text_len as f64
        },
        counts,
        comma_count,
        class_id: extract_class_and_id(fragment),
    }
}

fn count_in_wrapper(wrapper: &Option<ElementRef>, selector: &Selector) -> usize {
    wrapper
        .as_ref()
        .map(|node| node.select(selector).count())
        .unwrap_or(0)
}

fn count_multi_in_wrapper(wrapper: &Option<ElementRef>, selectors: &[&Selector]) -> usize {
    selectors
        .iter()
        .map(|selector| count_in_wrapper(wrapper, selector))
        .sum()
}

fn extract_class_and_id(fragment: &str) -> String {
    static CLASS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)class="([^"]*)""#).unwrap());
    static ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)id="([^"]*)""#).unwrap());

    let class = CLASS_REGEX
        .captures(fragment)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_lowercase())
        .unwrap_or_default();

    let id = ID_REGEX
        .captures(fragment)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_lowercase())
        .unwrap_or_default();

    format!("{class} {id}").trim().to_string()
}

/// Replace consecutive BR tags with paragraph tags
///
/// This converts content like:
/// ```html
/// <div>Text line 1<br><br>Text line 2</div>
/// ```
/// Into:
/// ```html
/// <div><p>Text line 1</p><p>Text line 2</p></div>
/// ```
///
/// This matches Mozilla's Readability _replaceBrs function
pub fn replace_brs(html: &str) -> String {
    let trimmed = html.trim();

    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        if let Some((tag_name, attributes, inner_content, closing_tag)) = parse_element(trimmed) {
            if closing_tag == tag_name {
                let processed_inner = replace_brs_in_content(inner_content);
                if attributes.is_empty() {
                    return format!("<{tag_name}>{processed_inner}</{tag_name}>");
                } else {
                    return format!("<{tag_name}{attributes}>{processed_inner}</{tag_name}>");
                }
            }
        }
    }

    replace_brs_in_content(trimmed)
}

/// Parse an HTML element into (tag_name, attributes, inner_content, closing_tag)
fn parse_element(html: &str) -> Option<(&str, &str, &str, &str)> {
    let opening_end = html.find('>')?;
    let opening_tag = &html[1..opening_end];

    let (tag_name, attributes) = if let Some(space_pos) = opening_tag.find(char::is_whitespace) {
        let tag = &opening_tag[..space_pos];
        let attrs = &opening_tag[space_pos..];
        (tag, attrs)
    } else {
        (opening_tag, "")
    };

    let closing_tag_pattern = format!("</{tag_name}>");
    let closing_start = html.rfind(&closing_tag_pattern)?;
    let inner_content = &html[opening_end + 1..closing_start];
    let closing_tag_name = tag_name;

    Some((tag_name, attributes, inner_content, closing_tag_name))
}

/// Replace BRs in text/content (no wrapping element)
fn replace_brs_in_content(content: &str) -> String {
    let br_regex = regex::Regex::new(r"(?i)(<br\s*/?>(\s|&nbsp;?)*){2,}").unwrap();
    if !br_regex.is_match(content) {
        return content.to_string();
    }

    let parts: Vec<&str> = br_regex.split(content).collect();
    let paragraphs: Vec<String> = parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{p}</p>"))
        .collect();

    if paragraphs.is_empty() {
        String::new()
    } else {
        paragraphs.join("\n    ")
    }
}

/// Prepare document for readability processing
///
/// This function implements Mozilla's _prepDocument functionality:
/// - Remove script and style elements
/// - Replace font tags with span
/// - Unwrap noscript tags to reveal lazy-loaded images
/// - Remove form elements
///
/// This should be called BEFORE content extraction
pub fn prep_document(html: &str) -> String {
    let mut html = html.to_string();

    let script_regex = regex::Regex::new(r"(?i)<script\b[^>]*>[\s\S]*?</script>").unwrap();
    html = script_regex.replace_all(&html, "").to_string();

    let style_regex = regex::Regex::new(r"(?i)<style\b[^>]*>[\s\S]*?</style>").unwrap();
    html = style_regex.replace_all(&html, "").to_string();

    let font_open_regex = regex::Regex::new(r"<font\b").unwrap();
    html = font_open_regex.replace_all(&html, "<span").to_string();

    let font_close_regex = regex::Regex::new(r"</font>").unwrap();
    html = font_close_regex.replace_all(&html, "</span>").to_string();

    let noscript_regex = regex::Regex::new(r"(?is)<noscript\b[^>]*>(.*?)</noscript>").unwrap();
    html = noscript_regex
        .replace_all(&html, |caps: &regex::Captures| {
            let inner = &caps[1];
            if inner.contains("<img") {
                inner.to_string()
            } else {
                caps[0].to_string()
            }
        })
        .to_string();

    let form_regex = regex::Regex::new(r"(?i)<form\b[^>]*>[\s\S]*?</form>").unwrap();
    html = form_regex.replace_all(&html, "").to_string();

    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_nav_like_sections() {
        let html = r#"
            <div>
                <nav>Navigation</nav>
                <div class="navbar menu">Menu</div>
                <section id="sidebar">Sidebar content</section>
                <p>Main article text</p>
            </div>
        "#;

        let cleaned = remove_nav_like_sections(html);
        assert!(cleaned.contains("<p>Main article text</p>"));
        assert!(!cleaned.contains("<nav"));
        assert!(!cleaned.contains("navbar"));
        assert!(!cleaned.contains("sidebar"));
    }

    #[test]
    fn test_remove_conditionally_removes_nav_table() {
        let html = r##"
            <article>
                <table class="nav-table">
                    <tr><td><a href="#">Home</a></td></tr>
                    <tr><td><a href="#">About</a></td></tr>
                </table>
                <p>Main story starts here</p>
            </article>
        "##;

        let cleaned = remove_conditionally(html);
        assert!(!cleaned.contains("nav-table"));
        assert!(cleaned.contains("Main story starts here"));
    }

    #[test]
    fn test_replace_brs_simple() {
        let html = "Line 1<br><br>Line 2";
        let result = replace_brs(html);
        assert!(result.contains("<p>Line 1</p>"));
        assert!(result.contains("<p>Line 2</p>"));
    }

    #[test]
    fn test_replace_brs_with_whitespace() {
        let html = "Line 1<br> <br>Line 2";
        let result = replace_brs(html);
        assert!(result.contains("<p>Line 1</p>"));
        assert!(result.contains("<p>Line 2</p>"));
    }

    #[test]
    fn test_replace_brs_multiple() {
        let html = "Para 1<br><br>Para 2<br><br><br>Para 3";
        let result = replace_brs(html);
        assert!(result.contains("<p>Para 1</p>"));
        assert!(result.contains("<p>Para 2</p>"));
        assert!(result.contains("<p>Para 3</p>"));
    }

    #[test]
    fn test_replace_brs_no_doubles() {
        let html = "Line 1<br>Line 2";
        let result = replace_brs(html);
        assert!(result.contains("Line 1<br>Line 2"));
    }

    #[test]
    fn test_replace_brs_with_wrapper_div() {
        let html = "<div>Lorem ipsum<br/>dolor sit<br/> <br/><br/>amet, consectetur</div>";
        let result = replace_brs(html);
        assert!(result.starts_with("<div>"));
        assert!(result.ends_with("</div>"));
        assert!(result.contains("<p>Lorem ipsum<br/>dolor sit</p>"));
        assert!(result.contains("<p>amet, consectetur</p>"));
    }

    #[test]
    fn test_replace_brs_preserves_attributes() {
        let html = r#"<div class="content" id="main">Text 1<br><br>Text 2</div>"#;
        let result = replace_brs(html);
        assert!(result.contains("class=\"content\""));
        assert!(result.contains("id=\"main\""));
        assert!(result.contains("<p>Text 1</p>"));
        assert!(result.contains("<p>Text 2</p>"));
    }
}
