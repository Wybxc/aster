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

#let social-links() = [
  #metadata("./social-links.css") <aster-style>
  #html.elem("nav", attrs: ("aria-label": "Social links"))[
    #html.elem("ul")[
      #for social in settings.socials {
        html.elem("li")[
          #html.elem("a", attrs: (
            href: social.href,
            title: social.name,
            "aria-label": social.name,
          ))[#_icon(social.name)]
        ]
      }
    ]
  ]
]
