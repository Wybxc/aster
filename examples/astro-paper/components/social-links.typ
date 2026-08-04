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

#let social-links() = html.elem("div", attrs: (class: "social-links"))[
  #for social in settings.socials {
    html.elem("a", attrs: (
      href: social.href,
      class: "icon-button",
      title: social.name,
      "aria-label": social.name,
    ))[#_icon(social.name)]
  }
]
