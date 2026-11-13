//! Turns a puzzle page into markdown.
//!
//! Only the puzzle descriptions are kept: the `<article class="day-desc">`
//! blocks, one per part the page shows. They use a small set of tags, so
//! they are converted by hand instead of with a general converter, which
//! keeps the awkward cases under control: emphasis inside code marks the
//! important values, and markdown has no direct way to write it.

use std::sync::LazyLock;

use ego_tree::NodeRef;
use scraper::{ElementRef, Html, Node, Selector};
use thiserror::Error;

static DESCRIPTION: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("article.day-desc").expect("the selector is valid"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the puzzle page has no puzzle description; the site layout may have changed")]
pub struct NoDescription;

/// The puzzle descriptions on `html` as markdown. Links are made absolute
/// against `page_url`, the address the page was served from.
pub fn to_markdown(html: &str, page_url: &str) -> Result<String, NoDescription> {
    let document = Html::parse_document(html);
    let context = Context { page_url };

    let blocks: Vec<String> = document
        .select(&DESCRIPTION)
        .flat_map(|article| context.blocks(article))
        .collect();
    if blocks.is_empty() {
        return Err(NoDescription);
    }

    Ok(format!(
        "<!-- Puzzle text from {page_url}. Advent of Code asks that it is not published. -->\n\n{}\n",
        blocks.join("\n\n")
    ))
}

struct Context<'a> {
    page_url: &'a str,
}

impl Context<'_> {
    fn blocks(&self, parent: ElementRef<'_>) -> Vec<String> {
        parent
            .children()
            .filter_map(|node| match node.value() {
                Node::Element(_) => self.block(ElementRef::wrap(node)?),
                Node::Text(text) => {
                    Some(escape(&collapse(text))).filter(|text| !text.trim().is_empty())
                }
                _ => None,
            })
            .collect()
    }

    fn block(&self, element: ElementRef<'_>) -> Option<String> {
        let block = match element.value().name() {
            "h1" | "h2" | "h3" => format!("## {}", self.inline_of(element)),
            "pre" => fenced(&element.text().collect::<String>()),
            "ul" | "ol" => self.list(element, 0),
            _ => self.inline_of(element),
        };
        Some(block).filter(|block| !block.is_empty())
    }

    fn list(&self, list: ElementRef<'_>, depth: usize) -> String {
        let marker = if list.value().name() == "ol" {
            "1."
        } else {
            "-"
        };
        let indent = "  ".repeat(depth);

        list.child_elements()
            .filter(|item| item.value().name() == "li")
            .map(|item| {
                let text = self.inline(item.children().filter(|child| !is_list(*child)));
                let nested = item
                    .child_elements()
                    .filter(|child| is_list(**child))
                    .map(|child| format!("\n{}", self.list(child, depth + 1)));

                format!("{indent}{marker} {text}") + &nested.collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn inline_of(&self, element: ElementRef<'_>) -> String {
        self.inline(element.children())
    }

    fn inline<'n>(&self, nodes: impl Iterator<Item = NodeRef<'n, Node>>) -> String {
        let text: String = nodes.map(|node| self.inline_node(node)).collect();
        text.trim().to_owned()
    }

    fn inline_node(&self, node: NodeRef<'_, Node>) -> String {
        if let Node::Text(text) = node.value() {
            return escape(&collapse(text));
        }
        let Some(element) = ElementRef::wrap(node) else {
            return String::new();
        };

        match element.value().name() {
            "em" | "strong" | "b" | "i" => {
                let inner = self.inline_of(element);
                if inner.is_empty() {
                    inner
                } else {
                    format!("*{inner}*")
                }
            }
            "code" => {
                let span = code_span(&element.text().collect::<String>());
                // Emphasis inside code is how the puzzles point at the
                // value that matters.
                if element.child_elements().next().is_some() {
                    format!("**{span}**")
                } else {
                    span
                }
            }
            "a" => match element.value().attr("href") {
                Some(href) => format!("[{}]({})", self.inline_of(element), self.resolve(href)),
                None => self.inline_of(element),
            },
            "br" => "  \n".to_owned(),
            _ => element
                .children()
                .map(|child| self.inline_node(child))
                .collect(),
        }
    }

    fn resolve(&self, href: &str) -> String {
        if href.contains("://") {
            return href.to_owned();
        }
        if href.starts_with('#') {
            return format!("{}{href}", self.page_url);
        }

        let after_scheme = self.page_url.find("://").map_or(0, |at| at + 3);
        let origin_end = self.page_url[after_scheme..]
            .find('/')
            .map_or(self.page_url.len(), |at| after_scheme + at);

        if href.starts_with('/') {
            format!("{}{href}", &self.page_url[..origin_end])
        } else {
            let directory_end = self.page_url.rfind('/').filter(|&at| at >= origin_end);
            match directory_end {
                Some(at) => format!("{}/{href}", &self.page_url[..at]),
                None => format!("{}/{href}", &self.page_url[..origin_end]),
            }
        }
    }
}

fn is_list(node: NodeRef<'_, Node>) -> bool {
    matches!(node.value(), Node::Element(element) if matches!(element.name(), "ul" | "ol"))
}

fn collapse(text: &str) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !in_space {
                collapsed.push(' ');
            }
            in_space = true;
        } else {
            collapsed.push(c);
            in_space = false;
        }
    }
    collapsed
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '<') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

fn code_span(code: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(code) + 1);
    let pad = if code.starts_with('`') || code.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{code}{pad}{fence}")
}

fn fenced(code: &str) -> String {
    let fence = "`".repeat((longest_backtick_run(code) + 1).max(3));
    let newline = if code.ends_with('\n') { "" } else { "\n" };
    format!("{fence}\n{code}{newline}{fence}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE_URL: &str = "https://adventofcode.com/2026/day/3";

    /// Shaped like a real page after part 1 is solved, with made-up text:
    /// real puzzle text must not be published, tests included.
    const SOLVED_PART_1: &str = r#"<!DOCTYPE html>
<html><head><title>Day 3</title></head><body>
<header><h1 class="title-global"><a href="/">Advent of Code</a></h1></header>
<main>
<style>article *[title]{border-bottom:1px dotted #ffff66;}</style><article class="day-desc"><h2>--- Day 3: Test Puzzle ---</h2><p>The elves <em>need</em> to count <code>widgets</code>.</p>
<p>Count the widgets in this <a href="https://example.com/widgets" target="_blank">example</a>:</p>
<pre><code>1 2
  3
</code></pre>
<ul>
<li>Start at <code>0</code>.</li>
<li>End at <code><em>3</em></code>.</li>
</ul>
<p><em>How many widgets are there?</em></p>
</article>
<p>Your puzzle answer was <code>42</code>.</p><article class="day-desc"><h2 id="part2">--- Part Two ---</h2><p>Now count <em>every <code>widget</code> twice</em>, says the <span title="a hidden joke">manager</span>.</p>
</article>
<p>Both parts of this puzzle are complete!</p>
</main>
</body></html>
"#;

    #[test]
    fn converts_every_description_on_the_page_and_nothing_else() {
        let markdown = to_markdown(SOLVED_PART_1, PAGE_URL).unwrap();

        assert_eq!(
            markdown,
            r"<!-- Puzzle text from https://adventofcode.com/2026/day/3. Advent of Code asks that it is not published. -->

## --- Day 3: Test Puzzle ---

The elves *need* to count `widgets`.

Count the widgets in this [example](https://example.com/widgets):

```
1 2
  3
```

- Start at `0`.
- End at **`3`**.

*How many widgets are there?*

## --- Part Two ---

Now count *every `widget` twice*, says the manager.
"
        );
    }

    #[test]
    fn a_page_without_a_description_is_an_error() {
        let page = "<html><body><main><p>Please log in.</p></main></body></html>";

        assert_eq!(to_markdown(page, PAGE_URL), Err(NoDescription));
    }

    fn convert(article_body: &str) -> String {
        let page = format!(r#"<article class="day-desc">{article_body}</article>"#);
        let markdown = to_markdown(&page, PAGE_URL).unwrap();
        markdown
            .split_once("-->\n\n")
            .unwrap()
            .1
            .trim_end()
            .to_owned()
    }

    #[test]
    fn escapes_text_that_markdown_would_read_as_formatting() {
        assert_eq!(
            convert("<p>a*b_c [d] `e` &lt;f&gt; \\</p>"),
            r"a\*b\_c \[d\] \`e\` \<f> \\"
        );
    }

    #[test]
    fn keeps_code_exactly_and_picks_a_fence_that_fits() {
        assert_eq!(convert("<p><code>*a_b*</code></p>"), "`*a_b*`");
        assert_eq!(convert("<p><code>a`b</code></p>"), "``a`b``");
        assert_eq!(
            convert("<pre><code>```\n# *x*\n</code></pre>"),
            "````\n```\n# *x*\n````"
        );
    }

    #[test]
    fn keeps_the_layout_of_a_code_block_and_drops_its_highlighting() {
        assert_eq!(
            convert("<pre><code>..#\n.<em>#</em>.\n</code></pre>"),
            "```\n..#\n.#.\n```"
        );
    }

    #[test]
    fn indents_nested_lists() {
        assert_eq!(
            convert("<ul><li>a<ul><li>b</li></ul></li><li>c</li></ul>"),
            "- a\n  - b\n- c"
        );
    }

    #[test]
    fn makes_site_links_absolute() {
        let context = Context { page_url: PAGE_URL };

        assert_eq!(
            context.resolve("/2026/day/2"),
            "https://adventofcode.com/2026/day/2"
        );
        assert_eq!(
            context.resolve("3/input"),
            "https://adventofcode.com/2026/day/3/input"
        );
        assert_eq!(
            context.resolve("#part2"),
            "https://adventofcode.com/2026/day/3#part2"
        );
        assert_eq!(
            context.resolve("https://example.com/"),
            "https://example.com/"
        );
    }
}
