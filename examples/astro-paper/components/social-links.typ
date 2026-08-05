#import "/lib.typ": settings
#import "icons.typ": github-icon, linkedin-icon, mail-icon, x-icon

#let _icon(name) = {
  if name == "GitHub" {
    github-icon
  } else if name == "X" {
    x-icon
  } else if name == "LinkedIn" {
    linkedin-icon
  } else {
    mail-icon
  }
}

#let social-links() = {
  let items = settings.socials.map(social => {
    list.item(
      html.a(
        href: social.href,
        title: social.name,
        aria-label: social.name,
      )[#_icon(social.name)],
    )
  })
  [
    #metadata("./social-links.css") <aster-style>
    #html.nav(class: "social-links", aria-label: "Social links")[
      #list(..items)
    ]
  ]
}
