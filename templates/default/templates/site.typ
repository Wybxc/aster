#import "/components/navigation.typ": navigation
#import "/lib.typ": settings

#let site(
  title: settings.site.title,
  description: settings.site.description,
  body,
) = html.html({
  html.head[
    #html.meta(charset: "utf-8")
    #html.meta(name: "viewport", content: "width=device-width, initial-scale=1")
    #html.meta(name: "description", content: description)
    #html.title(title)
    #metadata("/styles/site.css") <style>
  ]
  html.body[
    #navigation()
    #html.elem("main")[#body]
    #html.elem("footer")[Built with Aster and Typst.]
  ]
})
