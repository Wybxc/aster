#import "/templates/site.typ": site

#show: site.with(title: "About me", description: "A little about the author.", active: "about")

#html.main[
  #html.img(class: "about-photo", src: "/assets/blog-placeholder-about.jpg", alt: "A colorful abstract portrait")
  #html.h1[About me]

  Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
  tempor incididunt ut labore et dolore magna aliqua. Vitae ultricies leo
  integer malesuada nunc vel risus commodo viverra.

  Adipiscing enim eu turpis egestas pretium. Euismod elementum nisi quis
  eleifend quam adipiscing. In hac habitasse platea dictumst vestibulum.

  = Skills

  - Clear writing and thoughtful design
  - Building useful software
  - Learning in public
]
