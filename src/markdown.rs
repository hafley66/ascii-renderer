use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use crate::content::*;

fn flush_block(blocks: &mut Vec<ContentBlock>, items: &mut Vec<ContentItem>) {
    if items.is_empty() { return; }
    blocks.push(ContentBlock {
        items: items.drain(..).collect(),
        padding: 1,
    });
}

pub fn parse_markdown(input: &str) -> Vec<ContentBlock> {
    let parser = Parser::new(input);
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut items: Vec<ContentItem> = Vec::new();
    let mut text_buf = String::new();
    let mut in_blockquote = false;

    for event in parser {
        match event {
            Event::Text(t) => text_buf.push_str(&t),
            Event::Code(c) => {
                text_buf.push('`');
                text_buf.push_str(&c);
                text_buf.push('`');
            }
            Event::SoftBreak => text_buf.push(' '),
            Event::HardBreak => {
                if !text_buf.is_empty() {
                    let line = text_buf.drain(..).collect::<String>();
                    items.push(ContentItem::Text(line));
                }
            }

            Event::Start(Tag::Heading { .. }) => {
                flush_block(&mut blocks, &mut items);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
            }
            Event::Start(Tag::Paragraph)
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Item)
            | Event::Start(Tag::CodeBlock(_)) => {}
            Event::Start(_) => {}

            Event::End(TagEnd::Heading(level)) => {
                let title = text_buf.drain(..).collect::<String>();
                if level == HeadingLevel::H1 {
                    items.push(ContentItem::Text(format!("「 {} 」", title.to_uppercase())));
                } else {
                    items.push(ContentItem::Text(title));
                }
                items.push(ContentItem::Rule);
            }
            Event::End(TagEnd::Paragraph) => {
                if !text_buf.is_empty() {
                    let line = text_buf.drain(..).collect::<String>();
                    if in_blockquote {
                        items.push(ContentItem::Text(format!("│ {}", line)));
                    } else {
                        items.push(ContentItem::Text(line));
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                let line = text_buf.drain(..).collect::<String>();
                if !line.is_empty() {
                    items.push(ContentItem::Text(format!("▪ {}", line)));
                }
            }
            Event::End(TagEnd::List(_)) => {}
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                let code = text_buf.drain(..).collect::<String>();
                for line in code.lines() {
                    items.push(ContentItem::Text(line.to_string()));
                }
            }
            Event::End(_) => {}

            Event::Rule => {
                flush_block(&mut blocks, &mut items);
                items.push(ContentItem::Rule);
                flush_block(&mut blocks, &mut items);
            }

            _ => {}
        }
    }

    if !text_buf.is_empty() {
        items.push(ContentItem::Text(text_buf));
    }
    flush_block(&mut blocks, &mut items);
    blocks
}
