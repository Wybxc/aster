#import "/templates/site.typ": site

#show: site.with(title: "About me", description: "A little about the author.", active: "about")

#html.main[
  #html.img(class: "about-photo", src: "/assets/about-studio.jpg", alt: "A bright studio desk beside a city window")
  #html.h1[About Field Notes]

  Field Notes is an independent example publication built to show how a small
  editorial site can be organized around content collections.

  The writing favors practical observations, durable tools, and explanations
  that remain useful after the latest release cycle has passed.

  = Skills

  - Clear writing and thoughtful design
  - Building useful software
  - Learning in public
]
