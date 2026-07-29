// Page template: blog index
//
// Syntax highlighting CSS variables are generated automatically by Aster
// from the tmTheme themes declared in aster.toml and injected directly
// into <head> as a separate stylesheet.

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
