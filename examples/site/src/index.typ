// Page template: blog index
// Theme: override --hl-{scope} CSS variables to customize syntax highlighting
// colors.  Inspect the generated HTML to see which scope names your code
// produces — run `grep -oE 'var\(--hl-[^)]+\)' dist/*.html | sort -u`.
// Common examples:
//   --hl-default          un-tokenised / plain text
//   --hl-keyword-control  keywords (if, let, fn, …)
//   --hl-string-quoted    string literals
//   --hl-comment-line     line comments
//   --hl-constant-numeric numbers
//   --hl-entity-name      function / method names
#let theme-css = "
:root {
  --hl-default: #24292e;
  --hl-keyword-operator: #d73a49;
  --hl-keyword-typst: #d73a49;
  --hl-keyword-control: #d73a49;
  --hl-string-quoted: #032f62;
  --hl-string-quoted-double: #032f62;
  --hl-comment-line: #6a737d;
  --hl-comment-typst: #6a737d;
  --hl-constant-numeric: #6f42c1;
  --hl-entity-name: #005cc5;
  --hl-entity-name-function: #005cc5;
  --hl-punctuation-typst: #24292e;
  --hl-markup-raw: #24292e;
}
@media (prefers-color-scheme: dark) {
  :root {
    --hl-default: #e1e4e8;
    --hl-keyword-operator: #f97583;
    --hl-keyword-typst: #f97583;
    --hl-keyword-control: #f97583;
    --hl-string-quoted: #79b8ff;
    --hl-string-quoted-double: #79b8ff;
    --hl-comment-line: #959da5;
    --hl-comment-typst: #959da5;
    --hl-constant-numeric: #b392f0;
    --hl-entity-name: #79b8ff;
    --hl-entity-name-function: #79b8ff;
    --hl-punctuation-typst: #e1e4e8;
    --hl-markup-raw: #e1e4e8;
  }
}
"

#import "/lib/aster/content.typ": get-collection, render

#html.html({
  html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.title("Aster Sample Site")
    #html.elem("style")[#text(theme-css)]
  ]
  html.body[
    = Aster Sample Site

    This site demonstrates Aster's content collections system.

    == Blog Posts

    #let posts = get-collection("blog")

    #for post in posts {
      html.article[
        #render(post)
      ]
    }
  ]
})
