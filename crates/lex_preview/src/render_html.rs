//! Renders the parsed HTML subset from [`crate::html_tree`] as native gpui
//! elements, approximating lex-babel's baseline stylesheet with theme-aware
//! colors and fonts.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Font, FontStyle, FontWeight, Hsla, InteractiveText, SharedString, StyledText,
    TextRun, UnderlineStyle, div, img, px, rems,
};
use settings::Settings as _;
use theme_settings::ThemeSettings;
use ui::prelude::*;

use crate::html_tree::{HtmlElement, HtmlNode, HtmlTag};

pub struct RenderContext {
    text_color: Hsla,
    muted_color: Hsla,
    link_color: Hsla,
    code_background: Hsla,
    border_color: Hsla,
    text_font: Font,
    code_font: Font,
    /// Directory of the source document, for resolving relative image paths.
    document_dir: Option<PathBuf>,
    next_id: usize,
}

impl RenderContext {
    pub fn new(document_dir: Option<PathBuf>, cx: &App) -> Self {
        let theme_settings = ThemeSettings::get_global(cx);
        let colors = cx.theme().colors();
        Self {
            text_color: colors.text,
            muted_color: colors.text_muted,
            link_color: colors.link_text_hover,
            code_background: colors.surface_background,
            border_color: colors.border,
            text_font: theme_settings.ui_font.clone(),
            code_font: theme_settings.buffer_font.clone(),
            document_dir,
            next_id: 0,
        }
    }

    fn next_element_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }
}

/// Accumulated inline content: one string with styled runs and clickable
/// link ranges, built by flattening an element's inline children.
#[derive(Default)]
struct InlineText {
    text: String,
    runs: Vec<(Range<usize>, InlineStyle)>,
    links: Vec<(Range<usize>, String)>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    code: bool,
    link: bool,
}

impl InlineText {
    fn push_text(&mut self, text: &str, style: InlineStyle) {
        // HTML whitespace semantics: collapse runs of whitespace to one
        // space, and drop leading whitespace at the start of a block.
        for character in text.chars() {
            if character.is_whitespace() {
                if !self.text.is_empty() && !self.text.ends_with([' ', '\n']) {
                    self.push_char(' ', style);
                }
            } else {
                self.push_char(character, style);
            }
        }
    }

    fn push_char(&mut self, character: char, style: InlineStyle) {
        let start = self.text.len();
        self.text.push(character);
        let end = self.text.len();
        if let Some((last_range, last_style)) = self.runs.last_mut()
            && *last_style == style
            && last_range.end == start
        {
            last_range.end = end;
        } else {
            self.runs.push((start..end, style));
        }
    }

    fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

pub fn render_nodes(nodes: &[HtmlNode], rcx: &mut RenderContext, cx: &mut App) -> Vec<AnyElement> {
    let mut elements = Vec::new();
    let mut pending_inline = InlineText::default();

    for node in nodes {
        match node {
            HtmlNode::Text(text) => pending_inline.push_text(text, InlineStyle::default()),
            HtmlNode::Element(element) => {
                if is_inline(element.tag) {
                    collect_inline(node, InlineStyle::default(), &mut pending_inline);
                } else {
                    flush_inline(&mut pending_inline, &mut elements, rcx);
                    if let Some(element) = render_block(element, rcx, cx) {
                        elements.push(element);
                    }
                }
            }
        }
    }
    flush_inline(&mut pending_inline, &mut elements, rcx);
    elements
}

fn flush_inline(inline: &mut InlineText, elements: &mut Vec<AnyElement>, rcx: &mut RenderContext) {
    let inline = std::mem::take(inline);
    if !inline.is_empty() {
        elements.push(build_inline_element(inline, 1.0, FontWeight::NORMAL, rcx));
    }
}

fn is_inline(tag: HtmlTag) -> bool {
    matches!(
        tag,
        HtmlTag::A | HtmlTag::Em | HtmlTag::Strong | HtmlTag::Code | HtmlTag::Span | HtmlTag::Br
    )
}

fn collect_inline(node: &HtmlNode, style: InlineStyle, out: &mut InlineText) {
    match node {
        HtmlNode::Text(text) => out.push_text(text, style),
        HtmlNode::Element(element) => {
            let mut style = style;
            match element.tag {
                HtmlTag::Strong => style.bold = true,
                HtmlTag::Em => style.italic = true,
                HtmlTag::Code => style.code = true,
                HtmlTag::A => style.link = true,
                HtmlTag::Br => {
                    out.push_char('\n', style);
                    return;
                }
                HtmlTag::Img => {
                    if let Some(alt) = &element.alt {
                        out.push_text(alt, style);
                    }
                    return;
                }
                _ => {}
            }

            let link_start = out.text.len();
            for child in &element.children {
                collect_inline(child, style, out);
            }
            if element.tag == HtmlTag::A
                && let Some(href) = &element.href
            {
                out.links.push((link_start..out.text.len(), href.clone()));
            }
        }
    }
}

fn build_inline_element(
    inline: InlineText,
    relative_size: f32,
    base_weight: FontWeight,
    rcx: &mut RenderContext,
) -> AnyElement {
    let mut text_runs = Vec::with_capacity(inline.runs.len());
    for (range, style) in &inline.runs {
        let mut font = if style.code {
            rcx.code_font.clone()
        } else {
            rcx.text_font.clone()
        };
        font.weight = if style.bold {
            FontWeight::BOLD
        } else {
            base_weight
        };
        if style.italic {
            font.style = FontStyle::Italic;
        }
        text_runs.push(TextRun {
            len: range.len(),
            font,
            color: if style.link {
                rcx.link_color
            } else {
                rcx.text_color
            },
            background_color: style.code.then_some(rcx.code_background),
            underline: style.link.then_some(UnderlineStyle {
                thickness: px(1.),
                ..Default::default()
            }),
            strikethrough: None,
        });
    }

    let styled_text = StyledText::new(SharedString::from(inline.text)).with_runs(text_runs);
    let element = if inline.links.is_empty() {
        styled_text.into_any_element()
    } else {
        let (ranges, urls): (Vec<_>, Vec<_>) = inline.links.into_iter().unzip();
        InteractiveText::new(("lex-preview-text", rcx.next_element_id()), styled_text)
            .on_click(ranges, move |index, _window, cx| {
                if let Some(url) = urls.get(index) {
                    cx.open_url(url);
                }
            })
            .into_any_element()
    };
    div()
        .text_size(rems(relative_size))
        .child(element)
        .into_any_element()
}

fn render_block(
    element: &HtmlElement,
    rcx: &mut RenderContext,
    cx: &mut App,
) -> Option<AnyElement> {
    if let Some(level) = element.tag.heading_level() {
        return Some(render_heading(element, level, rcx));
    }
    match element.tag {
        HtmlTag::P | HtmlTag::Dt | HtmlTag::Figcaption => {
            let mut inline = InlineText::default();
            for child in &element.children {
                collect_inline(child, InlineStyle::default(), &mut inline);
            }
            if inline.is_empty() {
                return None;
            }
            let element = match element.tag {
                // lex renders definition terms bold-italic.
                HtmlTag::Dt => {
                    let mut styled = InlineText::default();
                    styled.runs = inline
                        .runs
                        .into_iter()
                        .map(|(range, mut style)| {
                            style.bold = true;
                            style.italic = true;
                            (range, style)
                        })
                        .collect();
                    styled.text = inline.text;
                    styled.links = inline.links;
                    build_inline_element(styled, 1.0, FontWeight::NORMAL, rcx)
                }
                HtmlTag::Figcaption => div()
                    .text_color(rcx.muted_color)
                    .text_size(rems(0.875))
                    .child(build_inline_element(inline, 0.875, FontWeight::NORMAL, rcx))
                    .into_any_element(),
                _ => build_inline_element(inline, 1.0, FontWeight::NORMAL, rcx),
            };
            Some(element)
        }
        HtmlTag::Ul | HtmlTag::Ol => Some(render_list(element, rcx, cx)),
        HtmlTag::Dl => Some(
            v_flex()
                .gap_1()
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
        HtmlTag::Dd => Some(
            v_flex()
                .pl_5()
                .gap_1()
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
        HtmlTag::Blockquote => Some(
            v_flex()
                .pl_3()
                .gap_2()
                .border_l_2()
                .border_color(rcx.border_color)
                .text_color(rcx.muted_color)
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
        HtmlTag::Pre => Some(render_code_block(element, rcx)),
        HtmlTag::Table => Some(render_table(element, rcx)),
        HtmlTag::Figure => Some(
            v_flex()
                .gap_1()
                .my_2()
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
        HtmlTag::Img => Some(render_image(element, rcx)),
        HtmlTag::Hr => Some(
            div()
                .my_2()
                .h(px(1.))
                .w_full()
                .bg(rcx.border_color)
                .into_any_element(),
        ),
        HtmlTag::Section => Some(render_section(element, rcx, cx)),
        // Transparent containers: lex-document / lex-doc-header wrappers and
        // any element the exporter emits that we don't know yet.
        HtmlTag::Div | HtmlTag::Header | HtmlTag::Other | HtmlTag::Thead | HtmlTag::Tbody => Some(
            v_flex()
                .gap_3()
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
        _ => Some(
            v_flex()
                .gap_2()
                .children(render_nodes(&element.children, rcx, cx))
                .into_any_element(),
        ),
    }
}

fn render_heading(element: &HtmlElement, level: u8, rcx: &mut RenderContext) -> AnyElement {
    let mut inline = InlineText::default();
    for child in &element.children {
        collect_inline(child, InlineStyle::default(), &mut inline);
    }
    // lex's baseline stylesheet keeps headings at body size and distinguishes
    // them by weight; only the document title is set larger here so the
    // hierarchy reads at a glance in a narrow pane.
    let size = if element.has_class("lex-doc-title") {
        1.4
    } else {
        match level {
            1 => 1.25,
            2 => 1.1,
            _ => 1.0,
        }
    };
    let weight = if level >= 6 {
        FontWeight::SEMIBOLD
    } else {
        FontWeight::BOLD
    };
    div()
        .mt_2()
        .child(build_inline_element(inline, size, weight, rcx))
        .into_any_element()
}

fn render_list(element: &HtmlElement, rcx: &mut RenderContext, cx: &mut App) -> AnyElement {
    let ordered = element.tag == HtmlTag::Ol;
    let mut items = Vec::new();
    let mut index = 0usize;
    for child in &element.children {
        let HtmlNode::Element(item) = child else {
            continue;
        };
        if item.tag != HtmlTag::Li {
            continue;
        }
        index += 1;
        let marker: SharedString = if ordered {
            format!("{index}.").into()
        } else {
            "–".into()
        };
        items.push(
            h_flex()
                .items_start()
                .gap_2()
                .child(div().text_color(rcx.muted_color).flex_none().child(marker))
                .child(
                    v_flex()
                        .gap_1()
                        .flex_grow(1.)
                        .children(render_nodes(&item.children, rcx, cx)),
                )
                .into_any_element(),
        );
    }
    v_flex().pl_2().gap_1().children(items).into_any_element()
}

fn render_code_block(element: &HtmlElement, rcx: &mut RenderContext) -> AnyElement {
    // Code block content is taken verbatim (no whitespace collapsing).
    let mut code = String::new();
    collect_verbatim_text(&element.children, &mut code);
    let code = code.trim_end_matches('\n').to_string();

    v_flex()
        .gap_1()
        .p_3()
        .rounded_md()
        .bg(rcx.code_background)
        .font_family(rcx.code_font.family.clone())
        .text_size(rems(0.875))
        .when_some(element.data_language.clone(), |this, language| {
            this.child(
                div()
                    .text_color(rcx.muted_color)
                    .text_size(rems(0.75))
                    .child(SharedString::from(language)),
            )
        })
        .child(SharedString::from(code))
        .into_any_element()
}

fn collect_verbatim_text(nodes: &[HtmlNode], out: &mut String) {
    for node in nodes {
        match node {
            HtmlNode::Text(text) => out.push_str(text),
            HtmlNode::Element(element) => {
                if element.tag == HtmlTag::Br {
                    out.push('\n');
                } else {
                    collect_verbatim_text(&element.children, out);
                }
            }
        }
    }
}

fn render_table(element: &HtmlElement, rcx: &mut RenderContext) -> AnyElement {
    let mut rows = Vec::new();
    collect_table_rows(element, &mut rows);

    let mut row_elements = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let mut cells = Vec::new();
        for cell in row {
            let is_header = cell.tag == HtmlTag::Th;
            let mut inline = InlineText::default();
            for child in &cell.children {
                collect_inline(child, InlineStyle::default(), &mut inline);
            }
            if is_header {
                inline.runs = inline
                    .runs
                    .into_iter()
                    .map(|(range, mut style)| {
                        style.bold = true;
                        (range, style)
                    })
                    .collect();
            }
            cells.push(
                div()
                    .flex_1()
                    .px_2()
                    .py_1()
                    .child(build_inline_element(inline, 1.0, FontWeight::NORMAL, rcx))
                    .into_any_element(),
            );
        }
        row_elements.push(
            h_flex()
                .w_full()
                .when(row_index + 1 < rows.len(), |this| {
                    this.border_b_1().border_color(rcx.border_color)
                })
                .children(cells)
                .into_any_element(),
        );
    }
    v_flex()
        .my_2()
        .border_1()
        .border_color(rcx.border_color)
        .rounded_md()
        .children(row_elements)
        .into_any_element()
}

fn collect_table_rows<'a>(element: &'a HtmlElement, rows: &mut Vec<Vec<&'a HtmlElement>>) {
    for child in &element.children {
        let HtmlNode::Element(child) = child else {
            continue;
        };
        match child.tag {
            HtmlTag::Thead | HtmlTag::Tbody => collect_table_rows(child, rows),
            HtmlTag::Tr => {
                let cells = child
                    .children
                    .iter()
                    .filter_map(|node| match node {
                        HtmlNode::Element(cell)
                            if matches!(cell.tag, HtmlTag::Th | HtmlTag::Td) =>
                        {
                            Some(cell)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            _ => {}
        }
    }
}

fn render_image(element: &HtmlElement, rcx: &mut RenderContext) -> AnyElement {
    let resolved_path = element.src.as_ref().and_then(|src| {
        let path = PathBuf::from(src);
        let path = if path.is_absolute() {
            path
        } else {
            rcx.document_dir.as_ref()?.join(path)
        };
        path.is_file().then_some(path)
    });

    if let Some(path) = resolved_path {
        img(Arc::<std::path::Path>::from(path.as_path()))
            .max_w_full()
            .into_any_element()
    } else {
        let placeholder: SharedString = element
            .alt
            .clone()
            .or_else(|| element.src.clone())
            .unwrap_or_else(|| "image".to_string())
            .into();
        div()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rcx.border_color)
            .text_color(rcx.muted_color)
            .child(placeholder)
            .into_any_element()
    }
}

/// Sections indent their content under a flush heading, matching the
/// indentation-is-structure reading of a lex document.
fn render_section(element: &HtmlElement, rcx: &mut RenderContext, cx: &mut App) -> AnyElement {
    let mut heading = None;
    let mut content = Vec::new();
    for child in &element.children {
        if heading.is_none()
            && let HtmlNode::Element(child_element) = child
            && child_element.tag.heading_level().is_some()
        {
            heading = render_block(child_element, rcx, cx);
        } else {
            content.push(child.clone());
        }
    }

    v_flex()
        .mt_2()
        .gap_2()
        .children(heading)
        .child(
            v_flex()
                .pl_4()
                .gap_3()
                .children(render_nodes(&content, rcx, cx)),
        )
        .into_any_element()
}
