// Page template: blog index
#import "/lib/aster/content.typ": get-collection, render

#html.html({
  html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.title("Aster Sample Site")
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
