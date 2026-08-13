//! A `Send`-able tree of the bounded HTML subset that lex-babel's HTML
//! export emits, plus conversion from an HTML document string.
//!
//! The renderer only understands this subset; unknown elements are kept as
//! [`HtmlTag::Other`] and rendered as transparent containers so future
//! additions to the exporter degrade gracefully instead of disappearing.

use html5ever::driver::ParseOpts;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtmlTag {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Ul,
    Ol,
    Li,
    Dl,
    Dt,
    Dd,
    Blockquote,
    Pre,
    Code,
    Table,
    Thead,
    Tbody,
    Tr,
    Th,
    Td,
    Figure,
    Figcaption,
    Img,
    A,
    Em,
    Strong,
    Span,
    Div,
    Section,
    Header,
    Hr,
    Br,
    #[default]
    Other,
}

impl HtmlTag {
    fn from_name(name: &str) -> Self {
        match name {
            "h1" => Self::H1,
            "h2" => Self::H2,
            "h3" => Self::H3,
            "h4" => Self::H4,
            "h5" => Self::H5,
            "h6" => Self::H6,
            "p" => Self::P,
            "ul" => Self::Ul,
            "ol" => Self::Ol,
            "li" => Self::Li,
            "dl" => Self::Dl,
            "dt" => Self::Dt,
            "dd" => Self::Dd,
            "blockquote" => Self::Blockquote,
            "pre" => Self::Pre,
            "code" => Self::Code,
            "table" => Self::Table,
            "thead" => Self::Thead,
            "tbody" => Self::Tbody,
            "tr" => Self::Tr,
            "th" => Self::Th,
            "td" => Self::Td,
            "figure" => Self::Figure,
            "figcaption" => Self::Figcaption,
            "img" => Self::Img,
            "a" => Self::A,
            "em" => Self::Em,
            "strong" => Self::Strong,
            "span" => Self::Span,
            "div" => Self::Div,
            "section" => Self::Section,
            "header" => Self::Header,
            "hr" => Self::Hr,
            "br" => Self::Br,
            _ => Self::Other,
        }
    }

    /// Heading level for h1-h6, if this is a heading tag.
    pub fn heading_level(self) -> Option<u8> {
        match self {
            Self::H1 => Some(1),
            Self::H2 => Some(2),
            Self::H3 => Some(3),
            Self::H4 => Some(4),
            Self::H5 => Some(5),
            Self::H6 => Some(6),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HtmlElement {
    pub tag: HtmlTag,
    pub class: Option<String>,
    pub href: Option<String>,
    pub src: Option<String>,
    pub alt: Option<String>,
    pub data_language: Option<String>,
    pub children: Vec<HtmlNode>,
}

impl HtmlElement {
    pub fn has_class(&self, class: &str) -> bool {
        self.class
            .as_deref()
            .is_some_and(|classes| classes.split_ascii_whitespace().any(|c| c == class))
    }
}

#[derive(Debug, Clone)]
pub enum HtmlNode {
    Text(String),
    Element(HtmlElement),
}

/// Parses an HTML document and returns the converted children of `<body>`.
pub fn parse_html_document(html: &str) -> anyhow::Result<Vec<HtmlNode>> {
    let parse_options = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let dom = parse_document(RcDom::default(), parse_options)
        .from_utf8()
        .read_from(&mut html.as_bytes())?;

    let body = find_element(&dom.document, "body")
        .ok_or_else(|| anyhow::anyhow!("HTML document has no body"))?;
    Ok(convert_children(&body))
}

fn find_element(handle: &Handle, tag_name: &str) -> Option<Handle> {
    if let NodeData::Element { name, .. } = &handle.data
        && name.local.as_ref() == tag_name
    {
        return Some(handle.clone());
    }
    for child in handle.children.borrow().iter() {
        if let Some(found) = find_element(child, tag_name) {
            return Some(found);
        }
    }
    None
}

fn convert_children(handle: &Handle) -> Vec<HtmlNode> {
    let mut nodes = Vec::new();
    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                let text = contents.borrow().to_string();
                nodes.push(HtmlNode::Text(text));
            }
            NodeData::Element { name, attrs, .. } => {
                let tag_name = name.local.as_ref();
                // Non-content subtrees the exporter emits in <head>; skipped
                // defensively in case they ever appear in the body.
                if matches!(tag_name, "style" | "script" | "meta" | "link" | "title") {
                    continue;
                }
                let mut element = HtmlElement {
                    tag: HtmlTag::from_name(tag_name),
                    ..Default::default()
                };
                for attr in attrs.borrow().iter() {
                    let value = attr.value.to_string();
                    match attr.name.local.as_ref() {
                        "class" => element.class = Some(value),
                        "href" => element.href = Some(value),
                        "src" => element.src = Some(value),
                        "alt" => element.alt = Some(value),
                        "data-language" => element.data_language = Some(value),
                        _ => {}
                    }
                }
                element.children = convert_children(child);
                nodes.push(HtmlNode::Element(element));
            }
            _ => {}
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_body(html: &str) -> Vec<HtmlNode> {
        parse_html_document(html).expect("parse must succeed")
    }

    fn as_element(node: &HtmlNode) -> &HtmlElement {
        match node {
            HtmlNode::Element(element) => element,
            HtmlNode::Text(text) => panic!("expected element, got text {text:?}"),
        }
    }

    #[test]
    fn parses_lex_export_shape() {
        let html = r#"<!DOCTYPE html>
            <html><head><title>Doc</title><style>p { color: red; }</style></head>
            <body><div class="lex-document">
            <header class="lex-doc-header"><h1 class="lex-doc-title">Doc</h1></header>
            <p class="lex-paragraph">Hello <strong>bold</strong> and <em>italic</em>.</p>
            <section class="lex-session lex-session-2"><h2>Section:</h2>
            <ul class="lex-list"><li class="lex-list-item">one</li></ul></section>
            </div></body></html>"#;

        let nodes = parse_body(html);
        let document = as_element(
            nodes
                .iter()
                .find(|node| matches!(node, HtmlNode::Element(_)))
                .expect("has element"),
        );
        assert_eq!(document.tag, HtmlTag::Div);
        assert!(document.has_class("lex-document"));

        let elements: Vec<_> = document
            .children
            .iter()
            .filter_map(|node| match node {
                HtmlNode::Element(element) => Some(element),
                _ => None,
            })
            .collect();
        assert_eq!(elements[0].tag, HtmlTag::Header);
        assert_eq!(elements[1].tag, HtmlTag::P);
        assert_eq!(elements[2].tag, HtmlTag::Section);
        assert!(elements[2].has_class("lex-session"));
    }

    #[test]
    fn skips_head_only_tags_and_keeps_unknown_elements() {
        let html = "<body><style>x</style><widget><p>inner</p></widget></body>";
        let nodes = parse_body(html);
        let elements: Vec<_> = nodes
            .iter()
            .filter_map(|node| match node {
                HtmlNode::Element(element) => Some(element),
                _ => None,
            })
            .collect();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].tag, HtmlTag::Other);
        assert_eq!(as_element(&elements[0].children[0]).tag, HtmlTag::P);
    }

    #[test]
    fn captures_link_image_and_language_attributes() {
        let html = r#"<body>
            <p><a href="https://lex.ing">site</a></p>
            <pre class="lex-verbatim" data-language="rust"><code>fn main() {}</code></pre>
            <figure><img src="pic.png" alt="A picture"><figcaption>cap</figcaption></figure>
            </body>"#;
        let nodes = parse_body(html);
        let elements: Vec<_> = nodes
            .iter()
            .filter_map(|node| match node {
                HtmlNode::Element(element) => Some(element),
                _ => None,
            })
            .collect();

        let link = as_element(&elements[0].children[0]);
        assert_eq!(link.tag, HtmlTag::A);
        assert_eq!(link.href.as_deref(), Some("https://lex.ing"));

        assert_eq!(elements[1].data_language.as_deref(), Some("rust"));

        let image = as_element(&elements[2].children[0]);
        assert_eq!(image.tag, HtmlTag::Img);
        assert_eq!(image.src.as_deref(), Some("pic.png"));
        assert_eq!(image.alt.as_deref(), Some("A picture"));
    }
}
