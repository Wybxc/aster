#let welcome() = [
  #metadata(
    ```css
    #background {
      position: fixed;
      inset: 0;
      z-index: -1;
      width: 100%;
      height: 100%;
      object-fit: cover;
      filter: blur(100px);
    }

    #container {
      min-height: 100vh;
      font-family: Inter, Roboto, "Helvetica Neue", Arial, sans-serif;
    }

    #container main {
      display: flex;
      min-height: 100vh;
      justify-content: center;
    }

    #hero {
      display: flex;
      align-items: flex-start;
      flex-direction: column;
      justify-content: center;
      padding: 1rem;
    }

    #hero > a img {
      width: 7.2rem;
      height: auto;
    }

    #hero h1 {
      max-width: 42rem;
      margin: 0.35rem 0 0;
      color: #111827;
      font-size: 1.375rem;
      line-height: 1.45;
    }

    #hero h1 code {
      display: inline-block;
      border: 1px solid #f041ff;
      border-radius: 0.5rem;
      padding: 0.25rem 0.5rem;
      background: #f9d7ef;
      color: #b423a5;
    }

    #links {
      display: flex;
      flex-wrap: wrap;
      gap: 1rem;
      margin-top: 1rem;
    }

    #links a {
      display: inline-flex;
      min-height: 2.75rem;
      align-items: center;
      padding: 0.65rem 0.8rem;
      color: #111827;
      text-decoration: none;
    }

    #links .button {
      border-radius: 0.5rem;
      background: #3245ff;
      color: white;
      box-shadow: inset 0 -2px 0 rgb(0 0 0 / 24%);
    }

    #news {
      position: absolute;
      right: 1rem;
      bottom: 1rem;
      width: min(20rem, calc(100% - 2rem));
      border: 1px solid white;
      border-radius: 0.5rem;
      padding: 1rem;
      background: white;
      color: #111827;
      text-decoration: none;
      box-shadow: 0 10px 35px rgb(31 41 55 / 14%);
    }

    #news h2 {
      margin: 0;
      font-size: 1.1rem;
    }

    #news p {
      margin: 0.45rem 0 0;
      color: #4b5563;
      line-height: 1.5;
    }

    @media (max-width: 48rem) {
      #hero {
        display: block;
        padding-top: 12vh;
      }

      #news {
        position: fixed;
      }
    }

    @media (max-height: 26rem) {
      #news {
        display: none;
      }
    }
    ```
  ) <aster-style>
  #html.div(id: "container")[
    #html.img(id: "background", src: "/assets/background.svg", alt: "")
    #html.main[
      #html.section(id: "hero")[
        #html.a(href: "https://astro.build")[
          #html.img(src: "/assets/astro.svg", width: 115, height: 48, alt: "Astro Homepage")
        ]
        #html.h1[To get started, open the #html.code[pages] directory in your project.]
        #html.section(id: "links")[
          #html.a(class: "button", href: "https://github.com/Wybxc/aster")[Read Aster's source]
          #html.a(href: "https://typst.app/docs/")[Read the Typst docs]
        ]
      ]
    ]
    #html.a(id: "news", href: "https://github.com/Wybxc/aster")[
      #html.h2[Build static sites with Typst]
      #html.p[Aster combines file routes, content collections, CSS processing, and a live development server.]
    ]
  ]
]
