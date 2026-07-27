// Page template: blog index
//
// CSS variables for syntax highlighting are defined in external stylesheets
// under src/ (style.css + theme-light.css + theme-dark.css).  They are
// bundled and minified into dist/style.css by the CSS loader.

#import "/lib/aster/content.typ": get-collection, render

#html.html({
  html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.title("Aster Sample Site")
    #html.elem("link", attrs: (
      "rel": "css",
      "href": "style.css",
    ))
  ]
  html.body[
    = Aster Sample Site

    This site demonstrates Aster's content collections system.

    == Blog Posts

    #let posts = get-collection("blog")

    #for post in posts {
      html.article[
        #let meta = post.metadata
        #if meta != (:) {
          html.p[
            #meta.at("title", default: "Untitled") \
            Published: #meta.at("date", default: "unknown")
          ]
        }
        #render(post)
      ]
    }
  ]
})
