// Page template: blog index
// Theme: override --hl-N CSS variables to customize syntax highlighting
// colors for both light and dark mode.
#let theme-css = "
:root {
  --hl-0: #d73a49;
  --hl-1: #6f42c1;
  --hl-2: #032f62;
}
@media (prefers-color-scheme: dark) {
  :root {
    --hl-0: #f97583;
    --hl-1: #b392f0;
    --hl-2: #79b8ff;
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
