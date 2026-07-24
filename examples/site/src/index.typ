// Page template: blog index
#import "/lib/aster/content.typ": get-collection, render

= Aster Sample Site

This site demonstrates Aster's content collections system.

== Blog Posts

#let posts = get-collection("blog")

#for post in posts {
  html.article[
    #render(post)
  ]
}
